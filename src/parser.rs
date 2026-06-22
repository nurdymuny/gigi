//! GQL — Geometric Query Language v2.0 parser (§6).
//!
//! Maps geometric statements to GIGI engine operations:
//!
//!   **GQL Native:**
//!   BUNDLE name BASE (...) FIBER (...) → create bundle
//!   SECTION name (...) → insert record
//!   SECTIONS name (...) → batch insert
//!   SECTION name AT k=v → point query (O(1))
//!   SECTION name AT k=v PROJECT (...) → projected point query
//!   REDEFINE name AT k=v SET (...) → update
//!   RETRACT name AT k=v → delete
//!   COVER name ON f=v → range query (bitmap, O(|bucket|))
//!   COVER name WHERE cond → filtered query (scan, O(|n|))
//!   COVER name DISTINCT f → distinct values
//!   COVER name ALL → list all
//!   INTEGRATE name OVER f MEASURE agg(g) → GROUP BY aggregation
//!   PULLBACK name ALONG f ONTO name → join
//!   CURVATURE name → scalar curvature
//!   SPECTRAL name → spectral gap
//!   CONSISTENCY name → Čech H¹
//!   EXPLAIN (...) → query plan
//!   SHOW BUNDLES → list bundles
//!   DESCRIBE name → schema info
//!   COLLAPSE name → drop bundle
//!   HEALTH name → full diagnostic
//!   EXISTS SECTION name AT k=v → existence check
//!   ATLAS BEGIN / COMMIT / ROLLBACK → transaction control
//!   BEGIN [TRANSACTION] / COMMIT / ROLLBACK → bare-spelling aliases
//!
//!   **SQL Compat (backward-compatible):**
//!   CREATE BUNDLE → BUNDLE
//!   INSERT INTO → SECTION
//!   SELECT → COVER / SECTION AT

use std::collections::HashMap;

// ── LOOP_TRANSPORT supporting types (parser surface) ─────────────────
//
// Halcyon Part VI deliverable #2. v3.1.3 §4.4 grammar freezes the
// control manifold pair to (Q, BETA_WILSON); the supporting types live
// here at the parser surface so the `Statement::LoopTransport` variant
// is self-contained (mirrors how `crate::gauge::ProjectGaussConfig` etc
// live in the gauge module — parser-surface types pinned by the
// grammar live here, executor-surface types live in
// `crate::gauge::loop_transport`).

/// Control manifold spec for `LOOP_TRANSPORT`. v3.1.3 freezes the
/// `(Q, BETA_WILSON)` pair; broader manifolds may land in later
/// specs as additional variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlManifoldSpec {
    /// `(Q, BETA_WILSON)` — the only manifold v3.1.3 ships.
    QBetaWilson,
}

/// Seed bracket from `SEEDS [lo..hi]`. Inclusive both ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedRange {
    /// Low end (inclusive).
    pub lo: u64,
    /// High end (inclusive).
    pub hi: u64,
}

/// One `COMPUTE …` output identifier per v3.1.3 §4.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopTransportOutputId {
    /// `COMPUTE HOLONOMY_FORWARD`.
    HolonomyForward,
    /// `COMPUTE HOLONOMY_REVERSED`.
    HolonomyReversed,
    /// `COMPUTE TRACKING_ERROR_TRACE_Q`.
    TrackingErrorTraceQ,
    /// `COMPUTE TRACKING_ERROR_TRACE_BETA_W`.
    TrackingErrorTraceBetaW,
    /// `COMPUTE ADIABATICITY_CHECK`.
    AdiabaticityCheck,
}

/// One `RETURN` field identifier per v3.1.3 §4.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopTransportReturnId {
    /// `H_forward`.
    HForward,
    /// `H_reversed`.
    HReversed,
    /// `sigma_H_blocked`.
    SigmaHBlocked,
    /// `per_seed_H_forward`.
    PerSeedHForward,
    /// `per_seed_H_reversed`.
    PerSeedHReversed,
    /// `tracking_error_max_Q`.
    TrackingErrorMaxQ,
    /// `tracking_error_max_beta_W`.
    TrackingErrorMaxBetaW,
    /// `adiabaticity_check`.
    AdiabaticityCheck,
}

/// Optional `SHAM { … }` block argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum ShamArg {
    /// Boolean literal `TRUE` / `FALSE`.
    Bool(bool),
    /// Numeric literal.
    Number(f64),
    /// Bare identifier or quoted string payload.
    Text(String),
}

/// `SHAM { … }` block. VI.2 PARSES the block forward-compat for VI.4;
/// the executor REJECTS any non-empty flag list with
/// `UnrecognizedShamFlag`. Empty `SHAM { }` is the only VI.2-runnable
/// case.
#[derive(Debug, Clone, PartialEq)]
pub struct ShamBlock {
    /// Ordered (flag-name, arg) pairs. `arg` defaults to `Bool(true)`
    /// when the flag is bare (no `= …` clause).
    pub flags: Vec<(String, ShamArg)>,
}

/// `LOOP` body — face index or explicit vertex sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopBody {
    /// `LOOP name ON lattice FACE n` — closed face cycle.
    Face(usize),
    /// `LOOP name ON lattice EDGES (v0, v1, …)` — vertex sequence.
    /// May be open; the LOOP_TRANSPORT executor raises
    /// `LoopNotClosed` per the audit-story flag contract.
    Edges(Vec<usize>),
}

/// Parsed GQL statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // ── Schema ──
    CreateBundle {
        name: String,
        base_fields: Vec<FieldSpec>,
        fiber_fields: Vec<FieldSpec>,
        indexed: Vec<String>,
        encrypted: bool,
        adjacencies: Vec<AdjacencySpec>,
        invariants: Vec<InvariantSpec>,
        /// v0.2: source of the master seed used to derive the GaugeKey.
        /// Defaults to `Random` (server CSPRNG); `WITH ENCRYPTION SEED 'hex'`
        /// or `WITH ENCRYPTION SEED FROM ENV $NAME` overrides.
        seed_source: crate::types::EncryptionSeedSource,
    },
    Collapse {
        bundle: String,
    },
    Describe {
        bundle: String,
        verbose: bool,
    },
    ShowBundles,

    // ── Write ──
    Insert {
        bundle: String,
        columns: Vec<String>,
        values: Vec<Literal>,
    },
    BatchInsert {
        bundle: String,
        columns: Vec<String>,
        rows: Vec<Vec<Literal>>,
    },
    BatchSectionUpsert {
        bundle: String,
        columns: Vec<String>,
        rows: Vec<Vec<Literal>>,
    },
    SectionUpsert {
        bundle: String,
        columns: Vec<String>,
        values: Vec<Literal>,
    },
    Redefine {
        bundle: String,
        key: Vec<(String, Literal)>,
        sets: Vec<(String, Literal)>,
    },
    BulkRedefine {
        bundle: String,
        conditions: Vec<FilterCondition>,
        sets: Vec<(String, Literal)>,
    },
    Retract {
        bundle: String,
        key: Vec<(String, Literal)>,
    },
    BulkRetract {
        bundle: String,
        conditions: Vec<FilterCondition>,
    },

    // ── Point Query ──
    PointQuery {
        bundle: String,
        key: Vec<(String, Literal)>,
        project: Option<Vec<String>>,
    },
    ExistsSection {
        bundle: String,
        key: Vec<(String, Literal)>,
    },

    // ── Range/Cover Query ──
    Cover {
        bundle: String,
        on_conditions: Vec<FilterCondition>,
        where_conditions: Vec<FilterCondition>,
        or_groups: Vec<Vec<FilterCondition>>,
        distinct_field: Option<String>,
        project: Option<Vec<String>>,
        rank_by: Option<Vec<SortSpec>>,
        first: Option<usize>,
        skip: Option<usize>,
        all: bool,
        /// Ask G Phase 4 follow-up (PH15): EXCLUDING IN clauses
        /// composing with COVER. Same left-anti-join-by-base-PK
        /// semantics as HUNT. Empty `Vec` when no EXCLUDING IN clauses
        /// are present.
        ///
        /// Gated on the `patterns` feature flag — when the flag is off,
        /// this Vec is always empty (set by the parser) and the executor
        /// is a no-op over it.
        #[cfg(feature = "patterns")]
        excluding: Vec<String>,
    },

    // ── Aggregation ──
    Integrate {
        bundle: String,
        over: Option<String>,
        measures: Vec<MeasureSpec>,
    },

    // ── Joins ──
    Pullback {
        left: String,
        along: String,
        right: String,
        right_field: Option<String>,
        preserve_left: bool,
    },

    // ── SQL Compat: SELECT ──
    Select {
        bundle: String,
        columns: Vec<SelectCol>,
        condition: Option<Condition>,
        group_by: Option<String>,
    },

    // ── SQL Compat: JOIN ──
    Join {
        left: String,
        right: String,
        on_field: String,
        columns: Vec<SelectCol>,
    },

    // ── Analytics ──
    Curvature {
        bundle: String,
        fields: Vec<String>,
        by_field: Option<String>,
    },
    Spectral {
        bundle: String,
        full: bool,
    },
    Consistency {
        bundle: String,
        repair: bool,
    },
    /// COMPLETE ON bundle [WHERE ...] [METHOD ...] [MIN_CONFIDENCE n] [WITH ...]
    Complete {
        bundle: String,
        where_conditions: Vec<FilterCondition>,
        method: Option<String>,
        min_confidence: Option<f64>,
        with_provenance: bool,
        with_constraint_graph: bool,
    },
    /// PROPAGATE ON bundle ASSUMING key=val, key=val [SHOW NEWLY_DETERMINED]
    Propagate {
        bundle: String,
        assumptions: Vec<(String, Literal)>,
    },
    /// SUGGEST_ADJACENCY ON bundle [FIELDS f1,f2,...] [SAMPLE_SIZE n] [CANDIDATES k] MINIMIZING h1
    SuggestAdjacency {
        bundle: String,
        fields: Vec<String>,
        sample_size: usize,
        candidates: usize,
    },
    Health {
        bundle: String,
    },
    Betti {
        bundle: String,
    },
    Entropy {
        bundle: String,
    },
    FreeEnergy {
        bundle: String,
        tau: f64,
    },
    /// CAPACITY <bundle> [TOLERANCE τ] — Davis capacity C = τ/K.
    /// Standalone verb: returns C, K, confidence, and interpretation.
    /// τ defaults to 1.0 when not specified.
    Capacity {
        bundle: String,
        tau: f64,
    },
    /// HORIZON <bundle> [TOLERANCE τ] — holonomy horizon s_max = τ/(K·ℓ_c).
    /// Returns the maximum coherent context depth for this bundle's geometry.
    /// τ defaults to 1.0; ℓ_c estimated from the spectral gap with
    /// Welford-radius fallback when the spectral gap is degenerate
    /// (`HorizonConfig::default()` semantics). The optional
    /// `LENGTH_SCALE` keyword overrides the primary estimator:
    ///   * `LENGTH_SCALE SPECTRAL_GAP`       — heat-kernel default
    ///   * `LENGTH_SCALE WELFORD_RADIUS`     — sqrt of mean fiber variance
    ///   * `LENGTH_SCALE FIXED <f64>`        — caller-provided constant
    /// When `config` is `None`, the executor passes
    /// `HorizonConfig::default()`.
    Horizon {
        bundle: String,
        tau: f64,
        config: Option<crate::curvature::HorizonConfig>,
    },
    /// DEPTH <bundle> [K_METRIC f64] [K_CONNECTION f64]
    ///                 [LAMBDA1_TOPOLOGICAL f64] [LAMBDA1_CONNECTION f64]
    /// Encoding depth classification from K and λ₁. Returns I (tangent) /
    /// II (connection) / III (metric) / IV (topological).
    ///
    /// The four optional threshold keywords override `DepthConfig`
    /// fields individually; unspecified thresholds use the published
    /// defaults from Theorem 8.14. When `config` is `None`, the
    /// executor passes `DepthConfig::default()` to the classifier
    /// (byte-identical to the pre-config behavior).
    Depth {
        bundle: String,
        config: Option<crate::curvature::DepthConfig>,
    },
    /// PERCEIVE <bundle>
    ///   ROTATION (r00, r01, ..., r_{N²-1})  -- row-major dim²
    ///   VECTOR   (v0, v1, ..., v_{N-1})
    ///   [DIM N]                              -- inferred from VECTOR if omitted
    ///
    /// Davis PERCEIVE (Theorem 8.6 — Cognitive Geometry Correspondence).
    /// Returns the perception bias `‖R - I‖_F` as the scalar result;
    /// the full v_perceived vector is on the HTTP surface
    /// (POST /v1/bundles/{name}/perceive) and Rust API
    /// (`curvature::perceive`). GQL exposes the scalar because GQL
    /// scalars compose with the rest of the language (e.g. comparisons,
    /// EXPLAIN blocks); vector results are a wire-only surface.
    ///
    /// `dim` is optional: when omitted, taken as `vector.len()`. If
    /// provided, the parser still validates `rotation.len() == dim * dim`
    /// and `vector.len() == dim` at execution time (in the executor).
    Perceive {
        bundle: String,
        rotation: Vec<f64>,
        vector: Vec<f64>,
        dim: Option<usize>,
    },
    Geodesic {
        bundle: String,
        from_keys: Vec<(String, Literal)>,
        to_keys: Vec<(String, Literal)>,
        max_hops: usize,
        restrict_bundle: Option<String>,
    },
    MetricTensor {
        bundle: String,
    },
    Explain {
        inner: Box<Statement>,
    },

    // ── Transaction ──
    AtlasBegin,
    AtlasCommit,
    AtlasRollback,

    // ── v2.1: Access Control ──
    WeaveRole {
        name: String,
        password: Option<String>,
        inherits: Option<String>,
        superweave: bool,
    },
    UnweaveRole {
        name: String,
    },
    ShowRoles,
    Grant {
        operations: Vec<String>,
        bundle: String,
        role: String,
    },
    Revoke {
        operations: Vec<String>,
        bundle: String,
        role: String,
    },
    CreatePolicy {
        name: String,
        bundle: String,
        operations: Vec<String>,
        restrict_query: String,
        role: String,
    },
    DropPolicy {
        name: String,
        bundle: String,
    },
    ShowPolicies {
        bundle: String,
    },
    AuditOn {
        bundle: String,
        operations: Vec<String>,
    },
    AuditOff {
        bundle: String,
    },
    AuditShow {
        bundle: String,
        since: Option<String>,
        role: Option<String>,
    },

    // ── v2.1: Constraints ──
    GaugeConstrain {
        bundle: String,
        constraints: Vec<String>,
    },
    GaugeUnconstrain {
        bundle: String,
        constraint_name: String,
    },
    GaugeTest {
        bundle1: String,
        bundle2: String,
        fiber_fields: Vec<String>,
        around_field: String,
    },
    // ── Parallel Transport / Holonomy ──
    Transport {
        bundle: String,
        from_keys: Vec<(String, Literal)>,
        to_keys: Vec<(String, Literal)>,
        fiber_fields: Vec<String>,
    },
    /// S4 — SAMPLE_TRANSPORT: curvature-bounded neighborhood sampling.
    /// Returns `k` candidates from the fiber neighborhood of a source
    /// point within budget `tau` (max `d^2`), weighted by
    /// `exp(-beta * d^2)`.
    SampleTransport {
        bundle: String,
        from_keys: Vec<(String, Literal)>,
        fiber_fields: Vec<String>,
        budget: f64,
        k: usize,
        beta: Option<f64>,
        seed: Option<u64>,
    },
    /// C2 — Rodrigues-style parallel transport: rotation in the plane
    /// spanned by (FROM, TO) fiber vectors by a SPECIFIED angle.
    /// Returns the full N×N rotation matrix as `matrix_flat` (comma-
    /// separated, row-major). Different from `Transport`:
    ///   - Angle is supplied (not derived from inner product)
    ///   - Output is the matrix, not displacement / 2x2 block
    /// Pairs the Python C0 `rotation_in_plane(θ, c_a, c_b)` math.
    TransportRotation {
        bundle: String,
        from_keys: Vec<(String, Literal)>,
        to_keys: Vec<(String, Literal)>,
        fiber_fields: Vec<String>,
        angle: f64,
    },
    HolonomyFiber {
        bundle: String,
        fiber_fields: Vec<String>,
        around_field: String,
    },
    LocalHolonomy {
        bundle: String,
        near_point: Vec<(String, f64)>,
        near_radius: f64,
        near_metric: Option<String>,
        fiber_fields: Vec<String>,
        around_field: String,
    },
    // ── KL Divergence / Cross-bundle analytics ──
    Divergence {
        bundle_a: String,
        bundle_b: String,
    },
    // ── Spectral fiber analysis ──
    SpectralFiber {
        bundle: String,
        fiber_fields: Vec<String>,
        modes: usize,
    },
    // ── Ricci curvature (per-edge) ──
    Ricci {
        bundle: String,
    },
    // ── Coherence extensions (stubs) ──
    SectionCoherent {
        bundle: String,
    },
    ShowCharts {
        bundle: String,
    },
    ShowContradictions {
        bundle: String,
    },
    CollapseBranch {
        bundle: String,
    },
    Predict {
        bundle: String,
    },
    CoverGeodesic {
        bundle: String,
    },
    Why {
        bundle: String,
    },
    Implications {
        bundle: String,
    },
    ShowConstraints {
        bundle: String,
    },

    // ── v2.1: Maintenance ──
    Compact {
        bundle: String,
        analyze: bool,
    },
    Analyze {
        bundle: String,
        field: Option<String>,
        full: bool,
    },
    Vacuum {
        bundle: String,
        full: bool,
    },
    RebuildIndex {
        bundle: String,
        field: Option<String>,
    },
    CheckIntegrity {
        bundle: String,
    },
    Repair {
        bundle: String,
    },
    StorageInfo {
        bundle: String,
    },

    // ── v2.1: Session ──
    Set {
        key: String,
        value: Literal,
    },
    Reset {
        key: Option<String>,
    },
    ShowSettings,
    ShowSession,
    ShowCurrentRole,

    // ── v2.1: Data Movement ──
    Ingest {
        bundle: String,
        source: String,
        format: String,
    },
    Transplant {
        source: String,
        target: String,
        conditions: Vec<FilterCondition>,
        retract_source: bool,
    },
    GenerateBase {
        bundle: String,
        field: String,
        from_val: Literal,
        to_val: Literal,
        step: Literal,
    },
    Fill {
        bundle: String,
        field: String,
        method: String,
    },

    // ── v2.1: Prepared Statements ──
    Prepare {
        name: String,
        body: String,
    },
    Execute {
        name: String,
        params: Vec<Literal>,
    },
    Deallocate {
        name: Option<String>,
    },
    ShowPrepared,

    // ── v2.1: Backup / Restore ──
    Backup {
        bundle: Option<String>,
        path: String,
        compress: bool,
        incremental_since: Option<String>,
    },
    Restore {
        bundle: String,
        path: String,
        snapshot: Option<String>,
        rename: Option<String>,
    },
    VerifyBackup {
        path: String,
    },
    ShowBackups,

    // ── v2.1: Information Schema ──
    ShowFields {
        bundle: String,
    },
    ShowIndexes {
        bundle: String,
    },
    ShowMorphisms {
        bundle: String,
    },
    ShowTriggers {
        bundle: String,
    },
    ShowStatistics {
        bundle: String,
    },
    ShowGeometry {
        bundle: String,
    },
    ShowComments {
        bundle: String,
    },

    // ── v2.1: Comments ──
    CommentOn {
        target_type: String,
        target: String,
        comment: String,
    },

    // ── v2.1: Recursive ──
    Iterate {
        bundle: String,
        start_key: Vec<(String, Literal)>,
        step_field: String,
        max_depth: Option<usize>,
    },

    // ── v2.1: Triggers ──
    CreateTrigger {
        event: String,
        bundle: String,
        condition: Option<String>,
        action: String,
    },
    DropTrigger {
        name: String,
        bundle: String,
    },

    // ── Feature #6: Query Cache ──
    /// INVALIDATE CACHE [ON <bundle>]
    InvalidateCache {
        bundle: Option<String>,
    },

    // ── Sprint G: forward-secret key rotation ──
    /// `GAUGE <bundle> ROTATE_KEY FORWARD_SECRET [WITH ENCRYPTION SEED 'hex']`
    ///
    /// Atomically re-encrypts every record under a freshly-derived GaugeKey.
    /// The OLD key is dropped. After this call, ciphertext from before the
    /// rotation is no longer recoverable — even by an attacker who later
    /// learns the post-rotation key. That's the "forward-secret" guarantee.
    ///
    /// Sprint G core scope: fiber-side rekey (gauge seed rotates, every
    /// fiber re-encrypted with the new derived per-field transforms).
    /// Base-space hash seed rotation, WAL crash atomicity, and RG-flow
    /// snapshot coarsening are deferred to a follow-up.
    RotateKey {
        bundle: String,
        /// Source for the new seed. Default: `Random` (CSPRNG).
        new_seed_source: crate::types::EncryptionSeedSource,
    },

    // ── Sprint H: PROJECT INVARIANT — gauge-invariant query surface ──
    /// `PROJECT INVARIANT (expr1, expr2, ...) FROM <bundle> [WHERE <cond>]`
    ///
    /// Evaluates a list of geometric-invariant expressions against a bundle.
    /// The whitelist of allowed operations (curvature, confidence,
    /// spectral_gap, entropy, beta_0, beta_1, holonomy_avg) plus + and ×
    /// is enforced AT PARSE TIME. A query that compiles is structurally
    /// guaranteed never to reach a decryption code path: the GIGI Encrypt
    /// "0 bytes decrypted on invariant queries" claim is checked by
    /// `test_project_invariant_zero_decrypt_calls_in_execution_path`.
    ProjectInvariant {
        bundle: String,
        /// Each entry is one comma-separated expression in the SELECT-list
        /// position. The optional label is the canonical text of the
        /// expression (used as the JSON key in the response).
        expressions: Vec<(String, InvariantExpr)>,
        /// Optional WHERE filter — applied to records before invariant
        /// computation (delegated to the same predicate machinery as Cover).
        where_clause: Option<Vec<FilterCondition>>,
    },

    // ── Ask G: PATTERN_HUNT (per theory/scj/PATTERN_HUNT_SPEC_v0.1.md) ──
    //
    // Four AST variants, all behind `#[cfg(feature = "patterns")]`:
    //
    //   DefinePattern  — DEFINE PATTERN <name> AS <pred> [WEIGHT (...)] [USING (...)]
    //   DropPattern    — DROP PATTERN <name>
    //   ShowPatterns   — SHOW PATTERNS
    //   Hunt           — HUNT <name> IN <bundle> [EXCLUDING IN <b>]* [TOP n] [PROJECT ...]
    //
    // Phase 1 is parser-only — these variants exist so tests can
    // assert the grammar shape. Phase 2 wires the registry; Phase 3
    // wires the executor; Phase 4 wires the EXCLUDING IN anti-join.
    //
    // `weight` is stored as a raw token list (the spec's
    // `Option<Vec<String>>` shape) — the WeightExpr AST in §9.3 of
    // the spec lands in Phase 3 when the evaluator is wired.
    #[cfg(feature = "patterns")]
    DefinePattern {
        name: String,
        /// AND'd predicate clauses, reusing COVER's machinery.
        pred: Vec<FilterCondition>,
        /// OR'd alternative predicate groups (each inner Vec is an
        /// AND'd group; the outer Vec is the OR alternatives).
        or_groups: Vec<Vec<FilterCondition>>,
        /// Tokenized WEIGHT expression. `None` if WEIGHT clause is
        /// absent. Phase 3 wires the parser-to-WeightExpr step.
        weight: Option<Vec<String>>,
        /// Declared fiber field touch-set, used by the planner for
        /// index selection and decryption-scope minimization. Empty
        /// `Vec` when USING is absent.
        using_fields: Vec<String>,
        /// `DEFINE OR REPLACE PATTERN p ...` sets this to `true` —
        /// overwrites a pre-existing pattern of the same name silently.
        /// Without it, a name collision returns a typed error (PH6a).
        replace: bool,
    },
    #[cfg(feature = "patterns")]
    DropPattern { name: String },
    #[cfg(feature = "patterns")]
    ShowPatterns,
    #[cfg(feature = "patterns")]
    Hunt {
        /// Pattern name resolved at execution time via the registry.
        pattern: String,
        /// Target bundle.
        bundle: String,
        /// EXCLUDING IN clauses, in source order. Each is a bundle
        /// name; Phase 4 wires the Roaring bitmap difference.
        excluding: Vec<String>,
        /// Additional ON-style filter applied to the resolved pred.
        extra_on: Vec<FilterCondition>,
        /// Additional WHERE-style filter applied to the resolved pred.
        extra_where: Vec<FilterCondition>,
        /// Override the default `RANK BY _score DESC`.
        rank_by: Option<Vec<SortSpec>>,
        /// TOP N truncation. None = no truncation.
        top: Option<usize>,
        /// PROJECT field list. `_score` is always available.
        project: Option<Vec<String>>,
    },

    // ── LATTICE verb. Behind the `lattice` Cargo feature so the
    //    default-feature build's surface is byte-identical to the
    //    pre-LATTICE engine. General-purpose graph-topology primitive
    //    (Halcyon's Davis Wilson Lattice substrate is the first
    //    consumer but the type is not Halcyon-specific). The explicit
    //    form ships V/E/F by enumeration; the shorthand form names a
    //    canonical graph constructor (only `TRUNCATED_ICOSAHEDRON` at
    //    launch).
    //
    //    Stored as a string body the executor hands to
    //    `crate::lattice::Lattice::from_gql` (and the shorthand form
    //    to `crate::lattice::topology::truncated_icosahedron::buckyball`).
    //    This keeps the parser surface narrow — the Lattice algebra
    //    owns its own canonical re-emit form.
    /// `LATTICE name VERTICES n EDGES (…) FACES (…) [TOPOLOGY "S"];`
    #[cfg(feature = "lattice")]
    Lattice {
        /// User-facing lattice name (the `ident` after `LATTICE`).
        name: String,
        /// Verbatim GQL re-emit body (canonical form). The executor
        /// rebuilds the `Lattice` via `from_gql` so the parser and
        /// the Lattice algebra share one round-trip contract.
        gql: String,
    },
    /// `LATTICE name FROM TRUNCATED_ICOSAHEDRON [TOPOLOGY "S"];`
    ///
    /// Names a canonical graph constructor. Only
    /// `TRUNCATED_ICOSAHEDRON` is wired at launch; future canonical
    /// graphs (cubic-lattice / hexagonal / …) extend by adding
    /// constructor names here.
    #[cfg(feature = "lattice")]
    LatticeFromCanonical {
        /// User-facing lattice name.
        name: String,
        /// Canonical graph identifier (UPPER_SNAKE; e.g.
        /// `TRUNCATED_ICOSAHEDRON`).
        canonical: String,
        /// Optional topology hint.
        topology: Option<String>,
    },
    /// `SHOW LATTICE name;`
    #[cfg(feature = "lattice")]
    ShowLattice { name: String },

    // ── GAUGE_FIELD verb (gated by `gauge` feature). Closes TDD-HAL-II.5.
    //    Group-erased connection over a declared LATTICE. The parser
    //    accepts every variant `Group` knows about (SU(2)/SU(3)/U(1)/Z(N));
    //    the executor proceeds only for `Group::SU2` and returns
    //    `GaugeFieldError::UnsupportedGroup` for the rest (Bee's locked
    //    decision 6).
    /// `GAUGE_FIELD name ON LATTICE lat GROUP <g> INIT <init> [PERSIST];`
    #[cfg(feature = "gauge")]
    GaugeField {
        /// User-facing field name (the `ident` after `GAUGE_FIELD`).
        name: String,
        /// Name of the lattice this field is bound to.
        lattice: String,
        /// Structure-group tag (Bee's locked decision 6 — group-erased
        /// storage). Only `Group::SU2` proceeds to construction at
        /// launch; the others are accepted syntactically and rejected
        /// at executor time with `GaugeFieldError::UnsupportedGroup`.
        group: crate::gauge::Group,
        /// Initialization recipe — `IDENTITY` / `HAAR_RANDOM` /
        /// `FROM <ident>`.
        init: crate::gauge::GaugeFieldInit,
        /// Optional Haar seed (mandatory for `HAAR_RANDOM` per Bee's
        /// locked decision 1; the parser accepts a missing seed and the
        /// executor surfaces `GaugeFieldError::SeedRequired` so the
        /// error path stays uniform).
        seed: Option<u64>,
        /// `PERSIST` keyword → durable via `engine.declare_gauge_field_durable`.
        /// Default (no keyword) registers in-memory only.
        persist: bool,
    },
    /// `SHOW GAUGE_FIELD name [BUFFER];`
    #[cfg(feature = "gauge")]
    ShowGaugeField {
        /// Gauge-field name to introspect.
        name: String,
        /// `BUFFER` keyword → include the full `(n_edges, repr_dim)`
        /// quaternion matrix in the result row's `data` column. Without
        /// `BUFFER` the row carries only metadata
        /// (name/lattice/group/repr_dim/n_edges/init_kind/init_seed).
        with_buffer: bool,
    },

    // ── PLAQUETTE / Q_SURROGATE projections (gated by `gauge`).
    //    Closes TDD-HAL-III.6 parser half. SELECT-level desugaring of
    //    the two gauge-substrate observables: PLAQUETTE OF U reduces
    //    to `gauge::plaquette_{per_face,mean,sum}` depending on the
    //    surrounding aggregate (locked decision D7), Q_SURROGATE OF U
    //    reduces to `gauge::q_surrogate` (scalar f64, locked decision
    //    D6). The grammar is group-agnostic; the executor dispatches
    //    through `handle.group()` (locked decision D4 group-erasure
    //    surface).
    /// `SELECT [MEAN|SUM]? PLAQUETTE OF name;`
    #[cfg(feature = "gauge")]
    SelectPlaquette {
        /// Gauge-field name the plaquette walks against.
        field: String,
        /// Which reduction the SELECT projection asked for.
        reduction: crate::gauge::PlaquetteReduction,
    },
    /// `SELECT Q_SURROGATE OF name;`
    #[cfg(feature = "gauge")]
    SelectQSurrogate {
        /// Gauge-field name the angular accumulator walks against.
        field: String,
    },

    // ── GIBBS_SAMPLE verb (gated by `gauge`). Closes TDD-HAL-III.6
    //    parser + executor half. Group-agnostic Statement variant;
    //    the executor matches `handle.group()` to dispatch into
    //    `gauge::gibbs_sample` (SU(2) at launch). Future SU(3) /
    //    U(1) heatbaths ship by extending the executor group-match,
    //    not by changing the grammar.
    /// `GIBBS_SAMPLE name BETA β [N_SWEEPS n] [MEASURE_EVERY m]
    ///  [MEASURE (obs_list)] [SEED s];`
    #[cfg(feature = "gauge")]
    GibbsSample {
        /// Gauge-field name to sweep against (mutated in-place via the
        /// SU(2)-mut registry escape, locked decision D4).
        field: String,
        /// Inverse temperature β. Echoed back in the response
        /// diagnostics block.
        beta: f64,
        /// Number of sweeps to run. Defaults to 1 when the optional
        /// `N_SWEEPS` clause is omitted (matches the Halcyon mock).
        n_sweeps: usize,
        /// Measurement cadence. Defaults to 1 when omitted. `0`
        /// disables the measurement epilogue entirely.
        measure_every: usize,
        /// Observable list. Empty when the optional `MEASURE (…)`
        /// clause is omitted. The parser emits the group-agnostic
        /// `ObservableId` enum; the executor lowers each tag to its
        /// SU(2) primitive.
        measure: Vec<crate::gauge::ObservableId>,
        /// Optional seed. `None` parses successfully and the executor
        /// surfaces `GaugeFieldError::SeedRequired` (locked decision
        /// 1 — intra-binding bit-identity is the contract; every
        /// GIBBS_SAMPLE call must commit to a seed).
        seed: Option<u64>,
    },

    // ── Part IV / TDD-HAL-IV.7 — E_FIELD + SYMPLECTIC_FLOW + SHOW
    //    E_FIELD + SELECT H_TOTAL + SELECT GAUSS_RESIDUAL_MAX.
    //    All gated on `gauge`. Group-agnostic Statement shape; the
    //    executor matches on `handle.group()` and dispatches to the
    //    SU(2) sibling kernels (locked decision IV-B).
    /// `E_FIELD name ON GAUGE_FIELD U INIT <init> [SEED s];`
    #[cfg(feature = "gauge")]
    EField {
        /// User-facing E-field name (the `ident` after `E_FIELD`).
        name: String,
        /// Name of the U gauge field this E binds to.
        source_gauge_field: String,
        /// Initialization recipe — `ZERO` / `MAXWELL_BOLTZMANN BETA β`
        /// / `FROM other_e`.
        init: crate::gauge::EFieldInit,
        /// Optional seed (mandatory for `MAXWELL_BOLTZMANN` per IV-B;
        /// the parser accepts a missing seed and the executor surfaces
        /// `GaugeFieldError::SeedRequired`).
        seed: Option<u64>,
    },
    /// `SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA β DT dt N_STEPS n
    /// [PROJECT_GAUSS <clause>] [MEASURE_EVERY m] [MEASURE (obs_list)]
    /// [SEED s];`
    #[cfg(feature = "gauge")]
    SymplecticFlow {
        /// Output / read-back field name (the `ident` after
        /// `SYMPLECTIC_FLOW`). Mirrors the Halcyon mock shape — the
        /// flow mutates the SOURCE U in place and `field` is the
        /// echo'd name in the response.
        field: String,
        /// Name of the U field paired in the FROM clause.
        e_field: String,
        /// Inverse temperature β.
        beta: f64,
        /// KDK step size.
        dt: f64,
        /// Number of leapfrog steps.
        n_steps: usize,
        /// Optional Gauss projection cadence. `Some(cfg)` enables per-
        /// step projection (production-canonical, IV-D); `None`
        /// disables projection entirely. `PROJECT_GAUSS TRUE` yields
        /// `Some(ProjectGaussConfig::default())`; `PROJECT_GAUSS
        /// FALSE` yields `None`; struct-literal sugar yields
        /// `Some(cfg)` with the named fields overridden.
        project_gauss: Option<crate::gauge::ProjectGaussConfig>,
        /// Measurement cadence. Defaults to 1 when omitted. `0`
        /// disables the measurement epilogue entirely.
        measure_every: usize,
        /// Observable list. Empty when the optional `MEASURE (…)`
        /// clause is omitted.
        measure: Vec<crate::gauge::ObservableId>,
        /// Optional seed (echoed in diagnostics; reserved for
        /// stochastic kernels Part V+ might add).
        seed: Option<u64>,
    },
    /// `LOOP name ON lattice FACE n;` — register a closed face cycle.
    /// `LOOP name ON lattice EDGES (v0, v1, …);` — register an explicit
    /// vertex path (may be open; LOOP_TRANSPORT raises `LoopNotClosed`
    /// at executor entry per the gate doc §SHAM audit-story flag).
    ///
    /// Halcyon Part VI deliverable #2 (LOOP_TRANSPORT verb)
    /// pre-registration: `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3`
    /// §4.4 (Zenodo DOI 10.5281/zenodo.20785681).
    #[cfg(feature = "gauge")]
    LoopDecl {
        /// User-facing loop name (the `ident` in `LOOP ident ON …;`).
        name: String,
        /// Owning lattice name.
        lattice: String,
        /// Loop body — either a single face index or an explicit
        /// vertex sequence.
        body: LoopBody,
    },
    /// `LOOP_TRANSPORT lattice ALONG_LOOP loop_id CONTROL_MANIFOLD
    /// (Q, BETA_WILSON) ADIABATIC ... RAMP_RATE_Q ... RAMP_RATE_BETA_W
    /// ... DRIVE_OMEGA ... DRIVE_F0 ... N_DISCRETIZATION ...
    /// PIN_LAMBDA_Q ... PIN_LAMBDA_BETA_W ... EPS_Q ... EPS_BETA_W ...
    /// ALPHA_HALCYON ... TAU_0 ... BETA_TAU ... MU_BASELINE ...
    /// K_SPRING ... C_DAMP ... [BETA_WILSON_START ...] SEEDS [lo..hi]
    /// COMPUTE ... [SHAM { ... }] RETURN ...;`
    ///
    /// Halcyon Part VI deliverable #2 verb. Pre-registration:
    /// `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3` §4.4 (Zenodo DOI
    /// 10.5281/zenodo.20785681), gate doc
    /// `theory/halcyon/HALCYON_PART_VI_GATES.md` @ 9a73dc0.
    ///
    /// EVOLVING: this variant's contract is pinned by v3.1.3 §4.4 and
    /// may evolve across minor versions per `STABILITY_GUARANTEES.md`.
    #[cfg(feature = "gauge")]
    LoopTransport {
        // -- topology binding --
        /// Lattice name (first ident after `LOOP_TRANSPORT`).
        lattice: String,
        /// Loop id (`ALONG_LOOP loop_id`).
        loop_id: String,
        // -- parameter-space loop γ in Λ = (Q, β_W) --
        /// Control manifold; v3.1.3 freezes `(Q, BETA_WILSON)`.
        control_manifold: ControlManifoldSpec,
        /// `ADIABATIC TRUE/FALSE`.
        adiabatic: bool,
        /// `RAMP_RATE_Q`.
        ramp_rate_q: f64,
        /// `RAMP_RATE_BETA_W`.
        ramp_rate_beta_w: f64,
        /// `DRIVE_OMEGA`.
        drive_omega: f64,
        /// `DRIVE_F0`.
        drive_f0: f64,
        /// `N_DISCRETIZATION` (substep count).
        n_discretization: usize,
        // -- pin / penalty --
        /// `PIN_LAMBDA_Q`.
        pin_lambda_q: f64,
        /// `PIN_LAMBDA_BETA_W`.
        pin_lambda_beta_w: f64,
        /// `EPS_Q`.
        eps_q: f64,
        /// `EPS_BETA_W`.
        eps_beta_w: f64,
        // -- Halcyon clock --
        /// `ALPHA_HALCYON`.
        alpha_halcyon: f64,
        /// `TAU_0`.
        tau_0: f64,
        /// `BETA_TAU`.
        beta_tau: f64,
        // -- Brownian / spring --
        /// `MU_BASELINE`.
        mu_baseline: f64,
        /// `K_SPRING`.
        k_spring: f64,
        /// `C_DAMP`.
        c_damp: f64,
        /// `SEEDS [lo..hi]` (inclusive both ends).
        seeds: SeedRange,
        /// `COMPUTE …` clauses (each one adds an output id).
        compute: Vec<LoopTransportOutputId>,
        /// `RETURN …` field list.
        return_fields: Vec<LoopTransportReturnId>,
        /// `SHAM { … }` block. `None` when omitted; `Some(empty)` is a
        /// valid VI.2 case (parses, executes without dispatch); any
        /// non-empty flag list is rejected at executor entry with
        /// `UnrecognizedShamFlag`.
        sham: Option<ShamBlock>,
        /// `BETA_WILSON_START` — launch coordinate on the BETA_WILSON
        /// axis of the control manifold. Defaults to the regime
        /// midpoint 2.75 when the clause is omitted. Threaded through
        /// to the executor so VI.6b Fix #5's amplitude-based β_W
        /// validation can use the actual start (the previous
        /// validate-and-discard convention forced the executor to fall
        /// back to 2.75, which masked legitimate out-of-regime
        /// configurations on the direct-executor path).
        beta_wilson_start: f64,
    },
    /// `SHOW E_FIELD name [BUFFER];`
    #[cfg(feature = "gauge")]
    ShowEField {
        /// E-field name to introspect.
        name: String,
        /// `BUFFER` keyword → include the full `(n_edges, 4)` Lie
        /// buffer in the response.
        with_buffer: bool,
    },
    /// `SELECT H_TOTAL OF (U, E);`
    #[cfg(feature = "gauge")]
    SelectHTotal {
        /// Name of the gauge field U.
        gauge_field: String,
        /// Name of the E field E.
        e_field: String,
    },
    /// `SELECT GAUSS_RESIDUAL_MAX OF (U, E);`
    #[cfg(feature = "gauge")]
    SelectGaussResidualMax {
        /// Name of the gauge field U.
        gauge_field: String,
        /// Name of the E field E.
        e_field: String,
        /// Reduction (Covariant / Flat). Defaults to Covariant.
        reduction: crate::gauge::GaussReduction,
    },

    // ── Part V / TDD-HAL-V.2 — SNAPSHOT GAUGE_FIELD verb ────────────
    //
    // Closes the parser + embedded-executor half of the durable
    // post-thermalization buffer snapshot path
    // (HALCYON_PART_V_SNAPSHOT_GATES.md §3 + Bee's locked decisions
    // D-V-A/B/C/D ratified 2026-06-19). Group-agnostic Statement
    // variant; the executor dispatches into
    // `engine.snapshot_gauge_field_durable` which writes a
    // `WalEntry::GaugeFieldSnapshot` and returns the SHA-256 +
    // WAL offset back as the citation handle.
    /// `SNAPSHOT GAUGE_FIELD name PERSIST;`
    ///
    /// `persist` is always `true` at this gate — bare
    /// `SNAPSHOT GAUGE_FIELD U;` parse-errors per locked decision D-V-D
    /// ("expected PERSIST"). The field is here so the future
    /// `TRANSIENT` variant (deferred per spec §6) can flip the bit
    /// without grammar surgery.
    #[cfg(feature = "gauge")]
    Snapshot {
        /// Gauge-field name to snapshot. Must resolve through
        /// `gauge_registry::get` at executor time; an undeclared name
        /// surfaces `GaugeFieldError::FieldNotDeclared` so callers see
        /// a stable "is not declared" Display string.
        name: String,
        /// Always `true` at V.2. Reserved for the future TRANSIENT
        /// variant (spec §6) so the existing PERSIST-call sites stay
        /// byte-identical when TRANSIENT lands.
        persist: bool,
    },
}

/// Whitelisted gauge-invariant operations. Each maps to an existing
/// computation that operates on the bundle's geometric structure (base
/// points, curvature tensor, spectral data) without ever requiring
/// decryption of fiber values. Adding a new op here MUST come with a
/// regression test that asserts zero decrypt calls during evaluation.
///
/// Conspicuously absent from this enum:
///   - `Entropy` — current `spectral::entropy` impl uses `store.records()`
///     which decrypts on access. Spec roadmap: add a base-only iterator
///     that yields just BASE fields and reactivate. Until then, use the
///     top-level `ENTROPY <bundle>` GQL statement when fiber decryption
///     is acceptable.
///
/// Adding any op here without first making the underlying compute
/// decrypt-free will break `test_project_invariant_zero_decrypt_calls`.
#[derive(Debug, Clone, PartialEq)]
pub enum InvariantOp {
    /// Scalar curvature K. Computed from per-field stats (variance, range)
    /// which are precomputed and stored — no fiber-value access required.
    Curvature,
    /// Confidence ∈ (0, 1] derived from K. Pure function of curvature.
    Confidence,
    /// Davis Law capacity C = τ/K. Pure function of curvature with a
    /// schema-supplied tolerance scalar τ. Both inputs are gauge-invariant
    /// → C is gauge-invariant.
    Capacity { tau: f64 },
    /// Spectral gap λ₁ of the graph Laplacian. Operates on the base-point
    /// adjacency graph derived from indexed BASE fields — never reads
    /// fiber values.
    SpectralGap,
    /// β₀ — number of connected components of the base-point graph.
    Beta0,
    /// β₁ — number of independent cycles of the base-point graph.
    Beta1,
    /// Average holonomy magnitude over a deterministic sample of triangle
    /// loops. Computed from BASE points only — does NOT touch fiber values.
    /// (The `crate::curvature::holonomy` function on numeric loops touches
    /// fibers; this op uses a base-only variant added in Sprint H-ext2.)
    HolonomyAvg,
}

/// Closed under +, ×, and constants. The grammar restricts the operand
/// space to `InvariantOp` and constants — there is no syntactic path to
/// reference a fiber field by name from inside an `InvariantExpr`, so the
/// evaluator structurally cannot need to decrypt.
#[derive(Debug, Clone, PartialEq)]
pub enum InvariantExpr {
    Op(InvariantOp),
    Const(f64),
    Add(Box<InvariantExpr>, Box<InvariantExpr>),
    Mul(Box<InvariantExpr>, Box<InvariantExpr>),
}

/// Stable label for an invariant expression — used as the JSON key in
/// PROJECT INVARIANT responses. Crude but deterministic.
pub fn invariant_label(expr: &InvariantExpr) -> String {
    match expr {
        InvariantExpr::Op(op) => match op {
            InvariantOp::Curvature => "curvature".into(),
            InvariantOp::Confidence => "confidence".into(),
            InvariantOp::Capacity { tau } => format!("capacity({tau})"),
            InvariantOp::SpectralGap => "spectral_gap".into(),
            InvariantOp::Beta0 => "beta_0".into(),
            InvariantOp::Beta1 => "beta_1".into(),
            InvariantOp::HolonomyAvg => "holonomy_avg".into(),
        },
        InvariantExpr::Const(c) => format!("{c}"),
        InvariantExpr::Add(a, b) => format!("({} + {})", invariant_label(a), invariant_label(b)),
        InvariantExpr::Mul(a, b) => format!("({} * {})", invariant_label(a), invariant_label(b)),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub name: String,
    pub ftype: String,
    pub range: Option<f64>,
    pub default: Option<Literal>,
    pub auto_inc: bool,
    pub unique: bool,
    pub required: bool,
    /// v0.2: per-field encryption mode declared after `ENCRYPTED` keyword.
    /// Defaults to `EncryptionMode::None` for fields not declared encrypted.
    /// When the bundle-level `ENCRYPTED` shorthand is used (v0.1 syntax),
    /// the parser fills this with `EncryptionMode::default_for_type` after
    /// the field type is known.
    pub encryption: crate::types::EncryptionMode,
    /// v0.2 (Sprint E): isometric group name. Set when the field declares
    /// `ENCRYPTED ISOMETRIC GROUP <name>`. Fields sharing a group are encrypted
    /// jointly with one shared O(k) matrix.
    pub encryption_group: Option<String>,
}

/// Parsed invariant constraint: INVARIANT field = value +/- tol
#[derive(Debug, Clone, PartialEq)]
pub struct InvariantSpec {
    pub field: String,
    pub expected: f64,
    pub tol: f64,
}

/// Parsed adjacency declaration: ADJACENCY name ON ... WEIGHT w
#[derive(Debug, Clone, PartialEq)]
pub struct AdjacencySpec {
    pub name: String,
    pub kind: AdjacencySpecKind,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdjacencySpecKind {
    /// ON field = field
    Equality { field: String },
    /// ON field WITHIN radius
    Metric { field: String, radius: f64 },
    /// ON field ABOVE threshold
    Threshold { field: String, threshold: f64 },
    /// ON field_a TO field_b VIA transform_fn
    Transform {
        source_field: String,
        target_field: String,
        transform: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectCol {
    Name(String),
    Star,
    Agg(AggFunc, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Eq(String, Literal),
    Between(String, Literal, Literal),
    In(String, Vec<Literal>),
}

/// Filter condition for COVER WHERE / ON clauses.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterCondition {
    Eq(String, Literal),
    Neq(String, Literal),
    Gt(String, Literal),
    Gte(String, Literal),
    Lt(String, Literal),
    Lte(String, Literal),
    In(String, Vec<Literal>),
    NotIn(String, Vec<Literal>),
    Contains(String, String),
    StartsWith(String, String),
    EndsWith(String, String),
    Matches(String, String),
    Void(String),
    Defined(String),
    Between(String, Literal, Literal),
    Exists {
        cover_bundle: String,
        where_conds: Vec<FilterCondition>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SortSpec {
    pub field: String,
    pub desc: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureSpec {
    pub func: AggFunc,
    pub field: String,
    pub alias: Option<String>,
}

// ── Tokenizer ──

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Number(f64),
    Str(String),
    LParen,
    RParen,
    /// `{` — struct-literal opener (TDD-HAL-IV.7 PROJECT_GAUSS sugar).
    LBrace,
    /// `}` — struct-literal closer (TDD-HAL-IV.7 PROJECT_GAUSS sugar).
    RBrace,
    /// `[` — bracket opener (LOOP_TRANSPORT SEEDS [lo..hi]).
    LBracket,
    /// `]` — bracket closer.
    RBracket,
    /// `..` — range operator (LOOP_TRANSPORT SEEDS [lo..hi]).
    DotDot,
    Comma,
    Eq,
    Neq, // != or <>
    Gt,  // >
    Gte, // >=
    Lt,  // <
    Lte, // <=
    Star,
    Slash, // /
    Dot,
    Colon, // :
    Semicolon,
    Plus,
    Minus,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            // Line comments: -- ...
            '-' if i + 1 < chars.len() && chars[i + 1] == '-' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '{' => {
                tokens.push(Token::LBrace);
                i += 1;
            }
            '}' => {
                tokens.push(Token::RBrace);
                i += 1;
            }
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '.' => {
                // `..` range operator (LOOP_TRANSPORT SEEDS [lo..hi])
                // takes precedence over the bare `.` member access.
                if i + 1 < chars.len() && chars[i + 1] == '.' {
                    tokens.push(Token::DotDot);
                    i += 2;
                } else {
                    tokens.push(Token::Dot);
                    i += 1;
                }
            }
            ';' => {
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '!' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Neq);
                i += 2;
            }
            '<' if i + 1 < chars.len() && chars[i + 1] == '>' => {
                tokens.push(Token::Neq);
                i += 2;
            }
            '<' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Lte);
                i += 2;
            }
            '<' => {
                tokens.push(Token::Lt);
                i += 1;
            }
            '>' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Gte);
                i += 2;
            }
            '>' => {
                tokens.push(Token::Gt);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            '\'' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("Unterminated string literal".into());
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Str(s));
                i += 1;
            }
            '-' => {
                // Could be negative number or minus
                if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    let start = i;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '.' {
                        i += 1;
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    // Scientific-notation suffix `e[±]N` / `E[±]N`
                    // (TDD-HAL-IV.7 PROJECT_GAUSS sugar — `1e-12`,
                    // `1.5e10`, etc.). We require at least one digit
                    // after the optional sign so plain `1e` doesn't
                    // greedily eat an identifier.
                    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                        let exp_start = i;
                        let mut j = i + 1;
                        if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                            j += 1;
                        }
                        if j < chars.len() && chars[j].is_ascii_digit() {
                            i = j;
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                i += 1;
                            }
                        } else {
                            // No digits after `e` — back out and let
                            // the alphabetic arm tokenize `e…` as a
                            // word.
                            let _ = exp_start;
                        }
                    }
                    let s: String = chars[start..i].iter().collect();
                    let n: f64 = s.parse().map_err(|_| format!("Invalid number: {s}"))?;
                    tokens.push(Token::Number(n));
                } else {
                    tokens.push(Token::Minus);
                    i += 1;
                }
            }
            '0'..='9' => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                // Treat a single `.` as a decimal point ONLY if it's
                // NOT immediately followed by another `.` — the
                // `..` range operator (LOOP_TRANSPORT SEEDS
                // [lo..hi]) must not be eaten as a number's decimal.
                if i < chars.len()
                    && chars[i] == '.'
                    && !(i + 1 < chars.len() && chars[i + 1] == '.')
                {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                // Scientific-notation suffix `e[±]N` / `E[±]N`
                // (TDD-HAL-IV.7 PROJECT_GAUSS sugar).
                if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                    let mut j = i + 1;
                    if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                        j += 1;
                    }
                    if j < chars.len() && chars[j].is_ascii_digit() {
                        i = j;
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                let s: String = chars[start..i].iter().collect();
                let n: f64 = s.parse().map_err(|_| format!("Invalid number: {s}"))?;
                tokens.push(Token::Number(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Word(word));
            }
            '$' => {
                // Parameter placeholder: $1, $2, etc.
                let start = i;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Word(word));
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            c => return Err(format!("Unexpected character: {c}")),
        }
    }
    Ok(tokens)
}

// ── Parser ──

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect_word(&mut self) -> Result<String, String> {
        match self.advance() {
            Some(Token::Word(w)) => Ok(w),
            other => Err(format!("Expected identifier, got {other:?}")),
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), String> {
        match self.advance() {
            Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw) => Ok(()),
            other => Err(format!("Expected '{kw}', got {other:?}")),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        let t = self.advance();
        if t.as_ref() == Some(&expected) {
            Ok(())
        } else {
            Err(format!("Expected {expected:?}, got {t:?}"))
        }
    }

    fn is_keyword(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn expect_usize(&mut self) -> Result<usize, String> {
        match self.advance() {
            Some(Token::Number(n)) if n >= 0.0 && n.fract() == 0.0 => Ok(n as usize),
            Some(Token::Word(w)) => w
                .parse()
                .map_err(|_| format!("Expected positive integer, got '{w}'")),
            other => Err(format!("Expected positive integer, got {other:?}")),
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek(), Some(Token::Semicolon))
    }

    fn parse_literal(&mut self) -> Result<Literal, String> {
        match self.advance() {
            Some(Token::Number(n)) => {
                if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
                    Ok(Literal::Integer(n as i64))
                } else {
                    Ok(Literal::Float(n))
                }
            }
            Some(Token::Str(s)) => Ok(Literal::Text(s)),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("true") => Ok(Literal::Bool(true)),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("false") => Ok(Literal::Bool(false)),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("null") => Ok(Literal::Null),
            other => Err(format!("Expected literal, got {other:?}")),
        }
    }

    // ── Top-level dispatch ──

    fn parse(&mut self) -> Result<Statement, String> {
        let first = self.expect_word()?;
        match first.to_ascii_uppercase().as_str() {
            // SQL compat
            "CREATE" => self.parse_create_bundle(),
            "INSERT" => self.parse_sql_insert(),
            "SELECT" => {
                // TDD-HAL-III.6: PLAQUETTE / Q_SURROGATE projection
                // desugaring. The classic `parse_sql_select` requires
                // a `FROM bundle` clause; the gauge-substrate forms
                // (`SELECT PLAQUETTE OF U;`, `SELECT MEAN(PLAQUETTE OF
                // U);`, `SELECT SUM(PLAQUETTE OF U);`, `SELECT
                // Q_SURROGATE OF U;`) intercept here so we can emit
                // dedicated `Statement::SelectPlaquette` /
                // `Statement::SelectQSurrogate` variants and leave the
                // bundle-SELECT path untouched.
                #[cfg(feature = "gauge")]
                {
                    if let Some(stmt) = self.try_parse_gauge_select()? {
                        return Ok(stmt);
                    }
                }
                self.parse_sql_select()
            }

            // GQL native
            "BUNDLE" => self.parse_bundle(),
            "SECTION" => self.parse_section(),
            "SECTIONS" => self.parse_sections(),
            "REDEFINE" => self.parse_redefine(),
            "RETRACT" => self.parse_retract(),
            "COVER" => self.parse_cover(),
            // Ask G — Patterns (parser-only Phase 1; behind `patterns` flag).
            #[cfg(feature = "patterns")]
            "DEFINE" => self.parse_define_pattern(),
            #[cfg(feature = "patterns")]
            "HUNT" => self.parse_hunt(),
            // LATTICE verb. Feature-gated on `lattice` so the default
            // build's reserved-keyword surface is unchanged. General-
            // purpose graph-topology primitive (Halcyon's Davis Wilson
            // Lattice substrate is the first consumer).
            #[cfg(feature = "lattice")]
            "LATTICE" => self.parse_lattice(),
            // GAUGE_FIELD verb. Feature-gated on `gauge` so the default
            // build's reserved-keyword surface is unchanged. Closes
            // TDD-HAL-II.5 (parser + executor).
            #[cfg(feature = "gauge")]
            "GAUGE_FIELD" => self.parse_gauge_field(),
            // GIBBS_SAMPLE verb. Closes TDD-HAL-III.6 (parser +
            // executor). Group-agnostic Statement variant; the
            // executor dispatches into `gauge::gibbs_sample` for
            // SU(2). The /v1/gql HTTP route reaches GIBBS_SAMPLE
            // through this parser arm — that is the documented soft
            // edge (locked decision D5); the 46-minute thermalization
            // wall self-enforces.
            #[cfg(feature = "gauge")]
            "GIBBS_SAMPLE" => self.parse_gibbs_sample(),
            // E_FIELD / SYMPLECTIC_FLOW verbs. Closes TDD-HAL-IV.7
            // (parser + executor). Group-agnostic Statement variants;
            // the executor matches on `handle.group()` and dispatches
            // to SU(2) sibling kernels (locked decision IV-B).
            #[cfg(feature = "gauge")]
            "E_FIELD" => self.parse_e_field(),
            #[cfg(feature = "gauge")]
            "SYMPLECTIC_FLOW" => self.parse_symplectic_flow(),
            // LOOP_TRANSPORT verb. Halcyon Part VI deliverable #2.
            // Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3
            // §4.4 (Zenodo DOI 10.5281/zenodo.20785681).
            #[cfg(feature = "gauge")]
            "LOOP_TRANSPORT" => self.parse_loop_transport(),
            // LOOP declaration (companion to LOOP_TRANSPORT). Registers
            // a named loop (face cycle or explicit edge list) against
            // a lattice. LOOP_TRANSPORT's `ALONG_LOOP` clause resolves
            // names through this registry.
            #[cfg(feature = "gauge")]
            "LOOP" => self.parse_loop_decl(),
            // SNAPSHOT verb. Closes TDD-HAL-V.2 (parser + executor).
            // Grammar:
            //   SNAPSHOT GAUGE_FIELD ident PERSIST ;
            // PERSIST is REQUIRED per Bee's locked decision D-V-D so
            // every caller is already explicit — when TRANSIENT lands
            // later (spec §6) zero existing SNAPSHOT call drifts.
            #[cfg(feature = "gauge")]
            "SNAPSHOT" => self.parse_snapshot(),
            "INTEGRATE" => self.parse_integrate(),
            "PULLBACK" => self.parse_pullback(),
            "COLLAPSE" => {
                let name = self.expect_word()?;
                Ok(Statement::Collapse { bundle: name })
            }
            "EXPLAIN" => self.parse_explain(),
            "SHOW" => self.parse_show(),
            "DESCRIBE" => {
                let name = self.expect_word()?;
                let verbose = self.is_keyword("VERBOSE");
                if verbose {
                    self.advance();
                }
                Ok(Statement::Describe {
                    bundle: name,
                    verbose,
                })
            }
            "HEALTH" => {
                let name = self.expect_word()?;
                Ok(Statement::Health { bundle: name })
            }
            "EXISTS" => self.parse_exists(),
            "ATLAS" => self.parse_atlas(),
            // Bare transaction-control spellings (ATOMIC_SHEAF_COMMIT_SPEC
            // §3.5/§7.2): aliases for ATLAS BEGIN / COMMIT / ROLLBACK.
            // Same statements, same semantics — transaction control is
            // handled at the transport layer, not the embedded executor.
            "BEGIN" => {
                if self.is_keyword("TRANSACTION") {
                    self.advance();
                }
                Ok(Statement::AtlasBegin)
            }
            "COMMIT" => Ok(Statement::AtlasCommit),
            "ROLLBACK" => Ok(Statement::AtlasRollback),

            // Analytics
            "CURVATURE" => self.parse_curvature(),
            "SPECTRAL" => self.parse_spectral(),
            "CONSISTENCY" => self.parse_consistency(),
            "BETTI" => self.parse_betti(),
            "ENTROPY" => self.parse_entropy(),
            "FREEENERGY" => self.parse_free_energy(),
            "CAPACITY"   => self.parse_capacity_stmt(),
            "HORIZON"    => self.parse_horizon_stmt(),
            "DEPTH"      => self.parse_depth_stmt(),
            "PERCEIVE"   => self.parse_perceive_stmt(),
            "GEODESIC" => self.parse_geodesic(),
            "METRIC" => self.parse_metric_tensor(),
            "COMPLETE" => self.parse_complete(),
            "PROPAGATE" => self.parse_propagate(),
            "SUGGEST_ADJACENCY" => self.parse_suggest_adjacency(),
            // Sprint H: PROJECT INVARIANT — gauge-invariant query surface
            "PROJECT" => self.parse_project_invariant(),

            // v2.1: Access Control
            "WEAVE" => self.parse_weave(),
            "UNWEAVE" => self.parse_unweave(),
            "GRANT" => self.parse_grant(),
            "REVOKE" => self.parse_revoke(),
            "POLICY" => self.parse_policy(),
            "DROP" => self.parse_drop(),
            "AUDIT" => self.parse_audit(),

            // v2.1: Constraints
            "GAUGE" => self.parse_gauge(),
            "TRANSPORT" => self.parse_transport(),
            "TRANSPORT_ROTATION" => self.parse_transport_rotation(),
            "SAMPLE_TRANSPORT" => self.parse_sample_transport(),
            "HOLONOMY" => self.parse_holonomy(),
            "DIVERGENCE" => {
                // DIVERGENCE bundle_a VS bundle_b
                let bundle_a = self.expect_word()?;
                self.expect_word()?; // VS
                let bundle_b = self.expect_word()?;
                Ok(Statement::Divergence { bundle_a, bundle_b })
            }

            // v2.1: Maintenance
            "COMPACT" => self.parse_compact(),
            "ANALYZE" => self.parse_analyze(),
            "VACUUM" => self.parse_vacuum(),
            "REBUILD" => self.parse_rebuild(),
            "CHECK" => self.parse_check(),
            "REPAIR" => {
                let name = self.expect_word()?;
                Ok(Statement::Repair { bundle: name })
            }
            "STORAGE" => {
                let name = self.expect_word()?;
                Ok(Statement::StorageInfo { bundle: name })
            }

            // v2.1: Session
            "SET" => self.parse_set(),
            "RESET" => self.parse_reset(),

            // v2.1: Data Movement
            "INGEST" => self.parse_ingest(),
            "TRANSPLANT" => self.parse_transplant(),
            "GENERATE" => self.parse_generate(),
            "FILL" => self.parse_fill(),

            // v2.1: Prepared Statements
            "PREPARE" => self.parse_prepare(),
            "EXECUTE" => self.parse_execute(),
            "DEALLOCATE" => self.parse_deallocate(),

            // v2.1: Backup / Restore
            "BACKUP" => self.parse_backup(),
            "RESTORE" => self.parse_restore(),
            "VERIFY" => self.parse_verify(),

            // v2.1: Comments
            "COMMENT" => self.parse_comment(),

            // v2.1: Recursive
            "ITERATE" => self.parse_iterate(),

            // v2.1: Triggers
            "BEFORE" | "AFTER" | "ON" => self.parse_trigger(&first),

            // Feature #6: Cache invalidation
            "INVALIDATE" => self.parse_invalidate_cache(),

            _ => Err(format!("Unknown statement: {first}")),
        }
    }

    // ── GQL: BUNDLE ──

    fn parse_bundle(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        // Optional opening paren (SQL-style) or keyword-style
        if matches!(self.peek(), Some(Token::LParen)) {
            // SQL-style: BUNDLE name (field TYPE ..., ...)
            return self.parse_bundle_fields_paren(name);
        }

        let mut base_fields = Vec::new();
        let mut fiber_fields = Vec::new();
        let mut indexed = Vec::new();

        // Keyword-style: BUNDLE name BASE (...) FIBER (...)
        if self.is_keyword("BASE") {
            self.advance();
            self.expect(Token::LParen)?;
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !base_fields.is_empty() {
                    self.expect(Token::Comma)?;
                }
                base_fields.push(self.parse_field_spec(&mut indexed)?);
            }
            self.expect(Token::RParen)?;
        }

        if self.is_keyword("FIBER") {
            self.advance();
            self.expect(Token::LParen)?;
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !fiber_fields.is_empty() {
                    self.expect(Token::Comma)?;
                }
                fiber_fields.push(self.parse_field_spec(&mut indexed)?);
            }
            self.expect(Token::RParen)?;
        }

        let encrypted = self.is_keyword("ENCRYPTED");
        if encrypted {
            self.advance();
        }

        // ADJACENCY clauses: ADJACENCY name ON field = field WEIGHT w
        // ADJACENCY () — shorthand for empty adjacency list
        let mut adjacencies = Vec::new();
        while self.is_keyword("ADJACENCY") {
            self.advance();
            // Allow ADJACENCY () as empty adjacency declaration
            if matches!(self.peek(), Some(Token::LParen)) {
                self.advance(); // (
                self.expect(Token::RParen)?;
                continue;
            }
            adjacencies.push(self.parse_adjacency_spec()?);
        }

        let invariants = self.parse_invariant_specs();
        let seed_source = self.parse_optional_encryption_seed_clause()?;

        Ok(Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
            invariants,
            seed_source,
        })
    }

    /// Parse `WITH ENCRYPTION SEED 'hex'` or `WITH ENCRYPTION SEED FROM ENV $NAME`
    /// if present after the field list. Returns the chosen source, defaulting to
    /// `Random` when no clause appears (v0.1 backwards-compat).
    fn parse_optional_encryption_seed_clause(
        &mut self,
    ) -> Result<crate::types::EncryptionSeedSource, String> {
        if !self.is_keyword("WITH") {
            return Ok(crate::types::EncryptionSeedSource::Random);
        }
        // Peek ahead: WITH ENCRYPTION SEED ...
        // If the next-next word isn't ENCRYPTION, leave the WITH for someone
        // else (defensive — other CREATE BUNDLE extensions might use WITH).
        // For now, if WITH is present, we expect ENCRYPTION SEED to follow.
        self.advance(); // consume WITH
        if !self.is_keyword("ENCRYPTION") {
            return Err("Expected ENCRYPTION after WITH in CREATE BUNDLE".to_string());
        }
        self.advance();
        if !self.is_keyword("SEED") {
            return Err("Expected SEED after ENCRYPTION".to_string());
        }
        self.advance();

        if self.is_keyword("FROM") {
            self.advance();
            if !self.is_keyword("ENV") {
                return Err("Expected ENV after FROM in WITH ENCRYPTION SEED FROM ENV".to_string());
            }
            self.advance();
            // Env var name: a bare word (e.g. JG_GIGI_SEED).
            let env_name = self.expect_word()?;
            return Ok(crate::types::EncryptionSeedSource::Env(env_name));
        }

        // Otherwise expect a hex literal (string in single quotes).
        match self.advance() {
            Some(Token::Str(hex)) => {
                if hex.len() != 64 {
                    return Err(format!(
                        "WITH ENCRYPTION SEED hex must be 64 characters (32 bytes), got {}",
                        hex.len()
                    ));
                }
                if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(
                        "WITH ENCRYPTION SEED hex must contain only [0-9a-fA-F]".to_string(),
                    );
                }
                Ok(crate::types::EncryptionSeedSource::Hex(hex))
            }
            other => Err(format!(
                "Expected 64-char hex string or FROM ENV after WITH ENCRYPTION SEED, got {other:?}"
            )),
        }
    }

    /// Parse: name ON field = field WEIGHT w | name ON field WITHIN r WEIGHT w | name ON field ABOVE t WEIGHT w
    fn parse_adjacency_spec(&mut self) -> Result<AdjacencySpec, String> {
        let adj_name = self.expect_word()?;
        self.expect_keyword("ON")?;
        let field = self.expect_word()?;

        let kind = if matches!(self.peek(), Some(Token::Eq)) {
            // Equality: ON field = field
            self.advance(); // consume =
            let _rhs = self.expect_word()?; // consume the repeated field name
            AdjacencySpecKind::Equality { field }
        } else if self.is_keyword("WITHIN") {
            self.advance();
            match self.advance() {
                Some(Token::Number(r)) => AdjacencySpecKind::Metric { field, radius: r },
                other => {
                    return Err(format!(
                        "Expected radius number after WITHIN, got {other:?}"
                    ))
                }
            }
        } else if self.is_keyword("ABOVE") {
            self.advance();
            match self.advance() {
                Some(Token::Number(t)) => AdjacencySpecKind::Threshold {
                    field,
                    threshold: t,
                },
                other => {
                    return Err(format!(
                        "Expected threshold number after ABOVE, got {other:?}"
                    ))
                }
            }
        } else if self.is_keyword("TO") {
            // Transform: ON field_a TO field_b VIA fn_name
            self.advance(); // consume TO
            let target_field = self.expect_word()?;
            self.expect_keyword("VIA")?;
            let transform = self.expect_word()?;
            AdjacencySpecKind::Transform {
                source_field: field,
                target_field,
                transform,
            }
        } else {
            return Err(format!(
                "Expected =, WITHIN, ABOVE, or TO after ADJACENCY ON {field}"
            ));
        };

        self.expect_keyword("WEIGHT")?;
        let weight = match self.advance() {
            Some(Token::Number(w)) => w,
            other => return Err(format!("Expected weight number, got {other:?}")),
        };

        Ok(AdjacencySpec {
            name: adj_name,
            kind,
            weight,
        })
    }

    fn parse_field_spec(&mut self, indexed: &mut Vec<String>) -> Result<FieldSpec, String> {
        let name = self.expect_word()?;
        let ftype = self.expect_word()?;
        let mut range = None;
        let mut default = None;
        let mut auto_inc = false;
        let mut unique = false;
        let mut required = false;
        // v0.2 per-field encryption mode. Defaults to None (plaintext) until
        // an `ENCRYPTED [MODE]` clause is seen on this field.
        let mut encryption = crate::types::EncryptionMode::None;
        // v0.2 (Sprint E): isometric group name (Some only when ISOMETRIC GROUP <name>).
        let mut encryption_group: Option<String> = None;

        loop {
            if self.is_keyword("RANGE") {
                self.advance();
                // RANGE n or RANGE(n)
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance();
                    match self.advance() {
                        Some(Token::Number(n)) => range = Some(n),
                        other => return Err(format!("Expected range value, got {other:?}")),
                    }
                    self.expect(Token::RParen)?;
                } else {
                    match self.advance() {
                        Some(Token::Number(n)) => range = Some(n),
                        other => return Err(format!("Expected range value, got {other:?}")),
                    }
                }
            } else if self.is_keyword("DEFAULT") {
                self.advance();
                default = Some(self.parse_literal()?);
            } else if self.is_keyword("AUTO") {
                self.advance();
                auto_inc = true;
            } else if self.is_keyword("UNIQUE") {
                self.advance();
                unique = true;
            } else if self.is_keyword("REQUIRED") {
                self.advance();
                required = true;
            } else if self.is_keyword("INDEX") {
                self.advance();
                indexed.push(name.clone());
            } else if self.is_keyword("ENCRYPTED") {
                self.advance();
                // Optional explicit mode keyword (v0.2). If absent, the mode
                // resolves to the type-default at schema-creation time
                // (see Statement::CreateBundle dispatch).
                let (mode, group) = self.parse_encryption_mode_and_group(&ftype)?;
                encryption = mode;
                encryption_group = group;
            } else {
                break;
            }
        }

        Ok(FieldSpec {
            name,
            ftype,
            range,
            default,
            auto_inc,
            unique,
            required,
            encryption,
            encryption_group,
        })
    }

    /// Parse the optional mode keyword that may follow `ENCRYPTED` on a field
    /// declaration. Recognized: `AFFINE`, `OPAQUE`, `INDEXED`, `PROBABILISTIC SIGMA <n>`,
    /// `ISOMETRIC`. If no recognized keyword follows, returns the type-default mode
    /// (numeric → Affine, text/binary → Opaque) so the bare `ENCRYPTED` shorthand
    /// from v0.1 keeps working.
    ///
    /// Validates type-mode compatibility:
    /// - `PROBABILISTIC` requires a numeric field type (NUMERIC, INTEGER, FLOAT, TIMESTAMP).
    /// - `INDEXED` requires a text-shaped type (TEXT, VARCHAR, STRING, CATEGORICAL).
    /// - `AFFINE` requires a numeric field type.
    /// - `OPAQUE` is universally valid.
    /// - `ISOMETRIC` is parsed but accepted only on grouped declarations
    ///   (group enforcement is in the caller; here we just return the mode).
    fn parse_encryption_mode_after_keyword(
        &mut self,
        ftype: &str,
    ) -> Result<crate::types::EncryptionMode, String> {
        // Backward-compat wrapper: drops the optional group name. Callers that
        // need the group should call parse_encryption_mode_and_group instead.
        let (mode, _group) = self.parse_encryption_mode_and_group(ftype)?;
        Ok(mode)
    }

    /// v0.2 (Sprint E): same as `parse_encryption_mode_after_keyword` but also
    /// returns the optional `GROUP <name>` clause that may follow `ISOMETRIC`.
    /// Returns `(EncryptionMode, Option<String>)` where the group is `Some`
    /// only for ISOMETRIC declarations that include `GROUP <name>`.
    fn parse_encryption_mode_and_group(
        &mut self,
        ftype: &str,
    ) -> Result<(crate::types::EncryptionMode, Option<String>), String> {
        let ftype_upper = ftype.to_ascii_uppercase();
        let is_numeric = matches!(
            ftype_upper.as_str(),
            "INT" | "INTEGER" | "NUMERIC" | "FLOAT" | "REAL" | "DOUBLE" | "TIMESTAMP" | "DATE"
        );
        let is_textish = matches!(
            ftype_upper.as_str(),
            "TEXT" | "VARCHAR" | "STRING" | "CATEGORICAL"
        );

        if self.is_keyword("AFFINE") {
            self.advance();
            if !is_numeric {
                return Err(format!(
                    "ENCRYPTED AFFINE requires a numeric field type; got {ftype}"
                ));
            }
            Ok((crate::types::EncryptionMode::Affine, None))
        } else if self.is_keyword("OPAQUE") {
            self.advance();
            Ok((crate::types::EncryptionMode::Opaque, None))
        } else if self.is_keyword("INDEXED") {
            self.advance();
            if !is_textish {
                return Err(format!(
                    "ENCRYPTED INDEXED is for high-cardinality TEXT/CATEGORICAL only; got {ftype}"
                ));
            }
            Ok((crate::types::EncryptionMode::Indexed, None))
        } else if self.is_keyword("PROBABILISTIC") {
            self.advance();
            if !is_numeric {
                return Err(format!(
                    "ENCRYPTED PROBABILISTIC requires a numeric field type; got {ftype}"
                ));
            }
            // Require SIGMA <n> to follow.
            if !self.is_keyword("SIGMA") {
                return Err(
                    "ENCRYPTED PROBABILISTIC requires `SIGMA <n>` to declare noise width".into(),
                );
            }
            self.advance();
            let sigma = match self.advance() {
                Some(Token::Number(n)) => n,
                other => {
                    return Err(format!(
                        "Expected numeric SIGMA value after PROBABILISTIC, got {other:?}"
                    ))
                }
            };
            if !(sigma > 0.0) {
                return Err(format!(
                    "SIGMA must be a positive number; got {sigma}"
                ));
            }
            Ok((crate::types::EncryptionMode::Probabilistic { sigma }, None))
        } else if self.is_keyword("ISOMETRIC") {
            self.advance();
            if !is_numeric {
                return Err(format!(
                    "ENCRYPTED ISOMETRIC requires a numeric field type; got {ftype}"
                ));
            }
            // Optional GROUP <name> clause. If absent, the field is its own
            // singleton group (Sprint E degenerate case → falls back to
            // Affine-like 1×1 identity matrix; still useful for distance
            // tests but offers no joint protection).
            let group = if self.is_keyword("GROUP") {
                self.advance();
                let g = self.expect_word()?;
                Some(g)
            } else {
                None
            };
            Ok((crate::types::EncryptionMode::Isometric, group))
        } else {
            // No explicit mode — fall back to the type-default. For numeric
            // fields this is Affine (v0.1 path); for text/binary it's Opaque.
            // We resolve here using a synthetic FieldType lookup based on the
            // declared GQL ftype string.
            let mode = match ftype_upper.as_str() {
                "INT" | "INTEGER" | "NUMERIC" | "FLOAT" | "REAL" | "DOUBLE" | "TIMESTAMP" | "DATE" => {
                    crate::types::EncryptionMode::Affine
                }
                _ => crate::types::EncryptionMode::Opaque,
            };
            Ok((mode, None))
        }
    }

    // ── GQL: SECTION (insert / point query) ──

    fn parse_section(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        // SECTION name AT k=v → point query
        if self.is_keyword("AT") {
            self.advance();
            let key = self.parse_kv_pairs()?;
            let mut project = None;
            if self.is_keyword("PROJECT") {
                self.advance();
                project = Some(self.parse_name_list()?);
            }
            return Ok(Statement::PointQuery {
                bundle: name,
                key,
                project,
            });
        }

        // SECTION name (...) [UPSERT] → insert
        self.expect(Token::LParen)?;
        let (columns, values) = self.parse_section_body()?;
        self.expect(Token::RParen)?;

        if self.is_keyword("UPSERT") {
            self.advance();
            return Ok(Statement::SectionUpsert {
                bundle: name,
                columns,
                values,
            });
        }

        Ok(Statement::Insert {
            bundle: name,
            columns,
            values,
        })
    }

    fn parse_section_body(&mut self) -> Result<(Vec<String>, Vec<Literal>), String> {
        let mut columns = Vec::new();
        let mut values = Vec::new();

        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !columns.is_empty() {
                self.expect(Token::Comma)?;
            }

            let col = self.expect_word()?;
            // Accept either : or = as separator
            if matches!(self.peek(), Some(Token::Colon)) || matches!(self.peek(), Some(Token::Eq)) {
                self.advance();
            } else {
                return Err(format!("Expected ':' or '=' after field name '{col}'"));
            }
            let val = self.parse_literal()?;
            columns.push(col);
            values.push(val);
        }

        Ok((columns, values))
    }

    // ── GQL: SECTIONS (batch insert) ──

    fn parse_sections(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        self.expect(Token::LParen)?;

        // Detect which of 3 patterns:
        // 1) Named: SECTIONS b (col: val, col: val, ...)
        // 2) Column-list + tuples: SECTIONS b (col, col, ...) (v, v, ...), (v, v, ...)
        // 3) Positional: SECTIONS b (v, v, v, ...)
        let named = self.pos + 1 < self.tokens.len()
            && matches!(self.tokens.get(self.pos), Some(Token::Word(_)))
            && matches!(
                self.tokens.get(self.pos + 1),
                Some(Token::Colon) | Some(Token::Eq)
            );

        // Check for column-list pattern: Word followed by , or ) (not : or =)
        let column_list = !named
            && matches!(self.tokens.get(self.pos), Some(Token::Word(_)))
            && matches!(
                self.tokens.get(self.pos + 1),
                Some(Token::Comma) | Some(Token::RParen)
            );

        let (columns, rows) = if named {
            // Pattern 1: Named key-value pairs, single row
            let mut columns = Vec::new();
            let mut current_row = Vec::new();

            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !columns.is_empty() || !current_row.is_empty() {
                    self.expect(Token::Comma)?;
                }
                let col = self.expect_word()?;
                if matches!(self.peek(), Some(Token::Colon))
                    || matches!(self.peek(), Some(Token::Eq))
                {
                    self.advance();
                }
                let val = self.parse_literal()?;
                columns.push(col);
                current_row.push(val);
            }
            self.expect(Token::RParen)?;
            (columns, vec![current_row])
        } else if column_list {
            // Pattern 2: SECTIONS b (col1, col2, ...) (v1, v2, ...), (v1, v2, ...)
            let mut columns = Vec::new();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !columns.is_empty() {
                    self.expect(Token::Comma)?;
                }
                columns.push(self.expect_word()?);
            }
            self.expect(Token::RParen)?;

            // Now parse value tuples
            let mut rows = Vec::new();
            loop {
                if !rows.is_empty() {
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance(); // consume comma between tuples
                    } else {
                        break;
                    }
                }
                if !matches!(self.peek(), Some(Token::LParen)) {
                    break;
                }
                self.expect(Token::LParen)?;
                let mut row = Vec::new();
                loop {
                    if matches!(self.peek(), Some(Token::RParen)) {
                        break;
                    }
                    if !row.is_empty() {
                        self.expect(Token::Comma)?;
                    }
                    row.push(self.parse_literal()?);
                }
                self.expect(Token::RParen)?;
                rows.push(row);
            }
            (columns, rows)
        } else {
            // Pattern 3: Positional values only, single row
            let mut all_values = Vec::new();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !all_values.is_empty() {
                    self.expect(Token::Comma)?;
                }
                all_values.push(self.parse_literal()?);
            }
            self.expect(Token::RParen)?;
            (vec![], vec![all_values])
        };

        // UPSERT suffix: SECTIONS b (...) UPSERT → batch upsert
        if self.is_keyword("UPSERT") {
            self.advance();
            return Ok(Statement::BatchSectionUpsert {
                bundle: name,
                columns,
                rows,
            });
        }

        Ok(Statement::BatchInsert {
            bundle: name,
            columns,
            rows,
        })
    }

    // ── GQL: REDEFINE (update) ──

    fn parse_redefine(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        if self.is_keyword("AT") {
            // Point update: REDEFINE name AT k=v SET (...)
            self.advance();
            let key = self.parse_kv_pairs()?;
            self.expect_keyword("SET")?;
            self.expect(Token::LParen)?;
            let sets = self.parse_kv_pairs_inner()?;
            self.expect(Token::RParen)?;
            Ok(Statement::Redefine {
                bundle: name,
                key,
                sets,
            })
        } else if self.is_keyword("ON") || self.is_keyword("WHERE") {
            // Bulk update: REDEFINE name ON/WHERE conditions SET (...)
            let conditions = self.parse_filter_conditions()?;
            self.expect_keyword("SET")?;
            self.expect(Token::LParen)?;
            let sets = self.parse_kv_pairs_inner()?;
            self.expect(Token::RParen)?;
            Ok(Statement::BulkRedefine {
                bundle: name,
                conditions,
                sets,
            })
        } else {
            Err("REDEFINE requires AT or ON/WHERE clause".into())
        }
    }

    // ── GQL: RETRACT (delete) ──

    fn parse_retract(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        if self.is_keyword("AT") {
            self.advance();
            let key = self.parse_kv_pairs()?;
            Ok(Statement::Retract { bundle: name, key })
        } else if self.is_keyword("ON") || self.is_keyword("WHERE") {
            let conditions = self.parse_filter_conditions()?;
            Ok(Statement::BulkRetract {
                bundle: name,
                conditions,
            })
        } else {
            Err("RETRACT requires AT or ON/WHERE clause".into())
        }
    }

    // ── GQL: COVER (range/filtered query) ──

    fn parse_cover(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        let mut on_conditions = Vec::new();
        let mut where_conditions = Vec::new();
        let mut or_groups = Vec::new();
        let mut distinct_field = None;
        let mut project = None;
        let mut rank_by = None;
        let mut first = None;
        let mut skip = None;
        let mut all = false;
        #[cfg(feature = "patterns")]
        let mut excluding: Vec<String> = Vec::new();

        // Parse optional clauses in any order
        loop {
            if self.at_end() {
                break;
            }

            if self.is_keyword("ALL") {
                self.advance();
                all = true;
            } else if self.is_keyword("ON") {
                self.advance();
                let conds = self.parse_filter_condition_list()?;
                on_conditions.extend(conds);
            } else if self.is_keyword("WHERE") {
                self.advance();
                let conds = self.parse_filter_condition_list()?;
                where_conditions.extend(conds);
            } else if self.is_keyword("OR") {
                self.advance();
                // Parse OR group
                let conds = self.parse_filter_condition_list()?;
                or_groups.push(conds);
            } else if self.is_keyword("DISTINCT") {
                self.advance();
                distinct_field = Some(self.expect_word()?);
            } else if self.is_keyword("PROJECT") {
                self.advance();
                project = Some(self.parse_name_list()?);
            } else if self.is_keyword("RANK") {
                self.advance();
                self.expect_keyword("BY")?;
                rank_by = Some(self.parse_sort_specs()?);
            } else if self.is_keyword("FIRST") {
                self.advance();
                first = Some(self.parse_usize()?);
            } else if self.is_keyword("SKIP") {
                self.advance();
                skip = Some(self.parse_usize()?);
            } else if cfg!(feature = "patterns") && self.is_keyword("EXCLUDING") {
                // Ask G — PH15: COVER accepts the same EXCLUDING IN clause
                // HUNT does. Behind `patterns` feature flag.
                self.advance();
                self.expect_keyword("IN")?;
                #[cfg(feature = "patterns")]
                {
                    excluding.push(self.expect_word()?);
                }
                #[cfg(not(feature = "patterns"))]
                {
                    return Err(
                        "EXCLUDING IN requires the `patterns` feature flag".to_string(),
                    );
                }
            } else {
                break;
            }
        }

        Ok(Statement::Cover {
            bundle: name,
            on_conditions,
            where_conditions,
            or_groups,
            distinct_field,
            project,
            rank_by,
            first,
            skip,
            all,
            #[cfg(feature = "patterns")]
            excluding,
        })
    }

    // ── LATTICE verb (gated by `lattice` feature) ──

    /// Parse a `LATTICE` statement (explicit or canonical-shorthand
    /// form). Closes the parser-surface half of TDD-HAL-I.7. The
    /// executor (`Statement::Lattice` / `Statement::LatticeFromCanonical`
    /// arms in `execute`) drives `crate::lattice` from here.
    ///
    /// Grammar (per `HALCYON_PART_I_GATES.md` Part I scope):
    ///
    /// ```ebnf
    /// lattice_stmt
    ///   : "LATTICE" ident "VERTICES" integer
    ///       "EDGES" "(" edge_list ")"
    ///       [ "FACES" "(" face_list ")" ]
    ///       [ "TOPOLOGY" string ]
    ///       ";"
    ///   | "LATTICE" ident "FROM" canonical_id
    ///       [ "TOPOLOGY" string ]
    ///       ";"
    ///   ;
    /// ```
    #[cfg(feature = "lattice")]
    fn parse_lattice(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        // Shorthand: LATTICE name FROM CANONICAL_ID [TOPOLOGY "..."];
        if self.is_keyword("FROM") {
            self.advance();
            let canonical = self.expect_word()?;
            let topology = if self.is_keyword("TOPOLOGY") {
                self.advance();
                match self.advance() {
                    Some(Token::Str(s)) => Some(s),
                    other => return Err(format!("Expected TOPOLOGY string, got {other:?}")),
                }
            } else {
                None
            };
            // Optional trailing semicolon — same lenience as the
            // rest of the parser.
            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.advance();
            }
            return Ok(Statement::LatticeFromCanonical {
                name,
                canonical,
                topology,
            });
        }

        // Explicit form: LATTICE name VERTICES n EDGES ((u,v),…)
        //   [FACES ((v0,v1,…),…)] [TOPOLOGY "S"];
        self.expect_keyword("VERTICES")?;
        let n_vertices = self.expect_usize()?;

        self.expect_keyword("EDGES")?;
        self.expect(Token::LParen)?;
        let mut edges: Vec<(usize, usize)> = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !edges.is_empty() {
                self.expect(Token::Comma)?;
            }
            self.expect(Token::LParen)?;
            let u = self.expect_usize()?;
            self.expect(Token::Comma)?;
            let v = self.expect_usize()?;
            self.expect(Token::RParen)?;
            edges.push((u, v));
        }
        self.expect(Token::RParen)?;

        let mut faces: Vec<Vec<usize>> = Vec::new();
        if self.is_keyword("FACES") {
            self.advance();
            self.expect(Token::LParen)?;
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !faces.is_empty() {
                    self.expect(Token::Comma)?;
                }
                self.expect(Token::LParen)?;
                let mut face: Vec<usize> = Vec::new();
                loop {
                    if matches!(self.peek(), Some(Token::RParen)) {
                        break;
                    }
                    if !face.is_empty() {
                        self.expect(Token::Comma)?;
                    }
                    face.push(self.expect_usize()?);
                }
                self.expect(Token::RParen)?;
                faces.push(face);
            }
            self.expect(Token::RParen)?;
        }

        let topology = if self.is_keyword("TOPOLOGY") {
            self.advance();
            match self.advance() {
                Some(Token::Str(s)) => Some(s),
                other => return Err(format!("Expected TOPOLOGY string, got {other:?}")),
            }
        } else {
            None
        };

        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }

        // Build the canonical GQL re-emit body via the Lattice
        // algebra's `to_gql`; the executor reads it back through
        // `from_gql`. One round-trip contract, owned by the
        // Lattice module.
        let lat = crate::lattice::Lattice::new(
            name.clone(),
            n_vertices,
            edges,
            faces,
            topology,
        );
        Ok(Statement::Lattice {
            name,
            gql: lat.to_gql(),
        })
    }

    // ── GAUGE_FIELD verb (gated by `gauge` feature) ──

    /// Parse a `GAUGE_FIELD` statement. Closes the parser half of
    /// TDD-HAL-II.5. The executor (`Statement::GaugeField` arm in
    /// `execute`) drives `crate::gauge::SU2GaugeField::new` +
    /// `crate::gauge::registry::register` (in-memory) or
    /// `engine.declare_gauge_field_durable` (PERSIST) from here.
    ///
    /// Grammar (per `GIGI_HALCYON_LATTICE_PRIMITIVES_SPRINT_SPEC.md`
    /// §3.P0.2, with the PERSIST extension from Bee's locked decision 3):
    ///
    /// ```ebnf
    /// gauge_field_stmt =
    ///   "GAUGE_FIELD" ident "ON" "LATTICE" ident "GROUP" group_label
    ///   "INIT" init_clause [ "PERSIST" ] ";"
    /// group_label = "SU(2)" | "SU(3)" | "U(1)" | "Z(" int ")"
    /// init_clause = "IDENTITY"
    ///             | "HAAR_RANDOM" [ "SEED" int ]
    ///             | "FROM" ident
    /// ```
    ///
    /// The parser accepts every `Group` variant syntactically — group
    /// erasure (Bee's locked decision 6) routes the non-SU(2) error
    /// path through the executor's `GaugeFieldError::UnsupportedGroup`
    /// arm so the failure has a stable `Display` impl Halcyon's G2.D
    /// regex anchor can match.
    #[cfg(feature = "gauge")]
    fn parse_gauge_field(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("ON")?;
        self.expect_keyword("LATTICE")?;
        let lattice = self.expect_word()?;
        self.expect_keyword("GROUP")?;
        let group = self.parse_group_label()?;
        self.expect_keyword("INIT")?;
        let (init, seed) = self.parse_init_clause()?;
        let persist = if self.is_keyword("PERSIST") {
            self.advance();
            true
        } else {
            false
        };
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Statement::GaugeField {
            name,
            lattice,
            group,
            init,
            seed,
            persist,
        })
    }

    /// Parse a `SNAPSHOT GAUGE_FIELD name PERSIST;` statement. Closes
    /// the parser half of TDD-HAL-V.2. The executor
    /// (`Statement::Snapshot` arm in `execute`) drives
    /// `engine.snapshot_gauge_field_durable(name)` which writes the WAL
    /// entry + returns the SHA-256 citation handle.
    ///
    /// Grammar (per `HALCYON_PART_V_SNAPSHOT_GATES.md` §3.P0.2 +
    /// Bee's locked decision D-V-D ratified 2026-06-19):
    ///
    /// ```ebnf
    /// snapshot_stmt = "SNAPSHOT" "GAUGE_FIELD" ident "PERSIST" ";"
    /// ```
    ///
    /// PERSIST is REQUIRED, not optional. Bare
    /// `SNAPSHOT GAUGE_FIELD U;` parse-errors with a message that
    /// names the missing keyword so the TRANSIENT variant (deferred to
    /// a future sprint per spec §6) can ship without flipping any
    /// existing call site.
    #[cfg(feature = "gauge")]
    fn parse_snapshot(&mut self) -> Result<Statement, String> {
        self.expect_keyword("GAUGE_FIELD")?;
        let name = self.expect_word()?;
        // PERSIST is required per Bee's locked decision D-V-D. The
        // error string names the deferred TRANSIENT variant so the
        // grammar's future shape is visible at the failure site.
        if !self.is_keyword("PERSIST") {
            return Err(format!(
                "SNAPSHOT GAUGE_FIELD {name}: expected PERSIST \
                 (TRANSIENT variant deferred to future sprint per \
                 HALCYON_PART_V_SNAPSHOT_GATES.md §6)"
            ));
        }
        self.advance();
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Statement::Snapshot {
            name,
            persist: true,
        })
    }

    /// Parse a `group_label` per the GAUGE_FIELD EBNF. The lexer
    /// already split `SU(2)` into `Word("SU"), LParen, Number(2),
    /// RParen` so we just reconstruct the variant.
    #[cfg(feature = "gauge")]
    fn parse_group_label(&mut self) -> Result<crate::gauge::Group, String> {
        let head = self.expect_word()?;
        match head.to_ascii_uppercase().as_str() {
            "SU" => {
                self.expect(Token::LParen)?;
                let n = self.expect_usize()?;
                self.expect(Token::RParen)?;
                match n {
                    2 => Ok(crate::gauge::Group::SU2),
                    3 => Ok(crate::gauge::Group::SU3),
                    other => Err(format!(
                        "GROUP SU({other}) not recognized (Part II ships SU(2)/SU(3) as group tags)"
                    )),
                }
            }
            "U" => {
                self.expect(Token::LParen)?;
                let n = self.expect_usize()?;
                self.expect(Token::RParen)?;
                match n {
                    1 => Ok(crate::gauge::Group::U1),
                    other => Err(format!(
                        "GROUP U({other}) not recognized (only U(1) is a recognized tag)"
                    )),
                }
            }
            "Z" => {
                self.expect(Token::LParen)?;
                let n = self.expect_usize()?;
                self.expect(Token::RParen)?;
                Ok(crate::gauge::Group::ZN { n: n as u32 })
            }
            other => Err(format!(
                "Expected group label (SU(2)/SU(3)/U(1)/Z(N)), got '{other}'"
            )),
        }
    }

    /// Parse an `init_clause` per the GAUGE_FIELD EBNF.
    #[cfg(feature = "gauge")]
    fn parse_init_clause(
        &mut self,
    ) -> Result<(crate::gauge::GaugeFieldInit, Option<u64>), String> {
        let head = self.expect_word()?;
        match head.to_ascii_uppercase().as_str() {
            "IDENTITY" => Ok((crate::gauge::GaugeFieldInit::Identity, None)),
            "HAAR_RANDOM" => {
                let seed = if self.is_keyword("SEED") {
                    self.advance();
                    Some(self.expect_usize()? as u64)
                } else {
                    None
                };
                Ok((crate::gauge::GaugeFieldInit::HaarRandom, seed))
            }
            "FROM" => {
                let src = self.expect_word()?;
                Ok((crate::gauge::GaugeFieldInit::FromField(src), None))
            }
            other => Err(format!(
                "Expected init clause (IDENTITY / HAAR_RANDOM / FROM), got '{other}'"
            )),
        }
    }

    /// Parse a `GIBBS_SAMPLE` statement. Closes the parser half of
    /// TDD-HAL-III.6. The executor (`Statement::GibbsSample` arm in
    /// `execute`) drives `crate::gauge::gibbs_sample::gibbs_sample`
    /// for SU(2) and returns a Rows envelope carrying the field name +
    /// measurement_history + diagnostics block from here.
    ///
    /// Grammar (per `GIGI_HALCYON_LATTICE_PRIMITIVES_SPRINT_SPEC.md`
    /// §P1.1, with the Q5 rename to `Q_SURROGATE`):
    ///
    /// ```ebnf
    /// gibbs_sample_stmt =
    ///   "GIBBS_SAMPLE" ident
    ///   "BETA" number
    ///   [ "N_SWEEPS" integer ]
    ///   [ "MEASURE_EVERY" integer ]
    ///   [ "MEASURE" "(" observable_list ")" ]
    ///   [ "SEED" integer ]
    ///   ";" ;
    /// observable_list = observable { "," observable } ;
    /// observable = "MEAN" "(" "PLAQUETTE" ")"
    ///            | "Q_SURROGATE"
    ///            | "H_TOTAL" | "GAUSS_RESIDUAL_MAX" | "EDGE_KINETIC"
    ///            | "VERTEX_GAUSS" | "ENERGY" ;
    /// ```
    ///
    /// Group erasure (Bee's locked decision D4): the grammar accepts
    /// every group; the executor routes to `gibbs_sample` only for
    /// SU(2). The optional `N_SWEEPS` / `MEASURE_EVERY` clauses default
    /// to 1 each (matches the Halcyon mock); a missing `MEASURE` clause
    /// emits an empty `Vec<ObservableId>`. Missing `SEED` parses
    /// successfully — the executor surfaces
    /// `GaugeFieldError::SeedRequired` so the typed-error contract
    /// stays uniform with GAUGE_FIELD INIT HAAR_RANDOM.
    #[cfg(feature = "gauge")]
    fn parse_gibbs_sample(&mut self) -> Result<Statement, String> {
        let field = self.expect_word()?;
        self.expect_keyword("BETA")?;
        let beta = self.expect_f64()?;
        let mut n_sweeps: usize = 1;
        let mut measure_every: usize = 1;
        let mut measure: Vec<crate::gauge::ObservableId> = Vec::new();
        let mut seed: Option<u64> = None;

        // Optional clauses may appear in any order. The Halcyon mock
        // and III.5 contract assume the canonical order BETA → N_SWEEPS
        // → MEASURE_EVERY → MEASURE → SEED, but the parser accepts any
        // permutation of the suffix clauses so HTTP round-trips through
        // hand-edited GQL stay forgiving.
        loop {
            if self.is_keyword("N_SWEEPS") {
                self.advance();
                n_sweeps = self.expect_usize()?;
            } else if self.is_keyword("MEASURE_EVERY") {
                self.advance();
                measure_every = self.expect_usize()?;
            } else if self.is_keyword("MEASURE") {
                self.advance();
                self.expect(Token::LParen)?;
                measure = self.parse_observable_list()?;
                self.expect(Token::RParen)?;
            } else if self.is_keyword("SEED") {
                self.advance();
                seed = Some(self.expect_usize()? as u64);
            } else {
                break;
            }
        }
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Statement::GibbsSample {
            field,
            beta,
            n_sweeps,
            measure_every,
            measure,
            seed,
        })
    }

    /// Parse an observable list — comma-separated entries inside
    /// `MEASURE (…)`. Each entry desugars to an `ObservableId` enum
    /// tag; pre-Part-IV variants parse successfully and the executor
    /// (or the III.5 primitive) surfaces a typed
    /// `PartIvObservableNotReady` error at sweep time.
    #[cfg(feature = "gauge")]
    fn parse_observable_list(&mut self) -> Result<Vec<crate::gauge::ObservableId>, String> {
        let mut out = Vec::new();
        loop {
            out.push(self.parse_observable()?);
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// Parse a single observable. The two SU(2)-substrate observables
    /// (`MEAN(PLAQUETTE)` and `Q_SURROGATE`) lower to
    /// `ObservableId::MeanPlaquette` / `ObservableId::QSurrogate`. The
    /// remaining Part-IV variants (`H_TOTAL`, `GAUSS_RESIDUAL_MAX`,
    /// `EDGE_KINETIC`, `VERTEX_GAUSS`, `ENERGY`) parse so the upstream
    /// regex anchors can match on the typed error string the executor
    /// surfaces.
    #[cfg(feature = "gauge")]
    fn parse_observable(&mut self) -> Result<crate::gauge::ObservableId, String> {
        let head = self.expect_word()?;
        match head.to_ascii_uppercase().as_str() {
            "MEAN" => {
                self.expect(Token::LParen)?;
                let inner = self.expect_word()?;
                self.expect(Token::RParen)?;
                if !inner.eq_ignore_ascii_case("PLAQUETTE") {
                    return Err(format!(
                        "Expected MEAN(PLAQUETTE), got MEAN({inner})"
                    ));
                }
                Ok(crate::gauge::ObservableId::MeanPlaquette)
            }
            "Q_SURROGATE" => Ok(crate::gauge::ObservableId::QSurrogate),
            "H_TOTAL" => Ok(crate::gauge::ObservableId::HTotal),
            "GAUSS_RESIDUAL_MAX" => Ok(crate::gauge::ObservableId::GaussResidualMax),
            "EDGE_KINETIC" => Ok(crate::gauge::ObservableId::EdgeKinetic),
            "VERTEX_GAUSS" => Ok(crate::gauge::ObservableId::VertexGauss),
            "ENERGY" => Ok(crate::gauge::ObservableId::Energy),
            other => Err(format!(
                "Expected observable (MEAN(PLAQUETTE) / Q_SURROGATE / H_TOTAL / \
                 GAUSS_RESIDUAL_MAX / EDGE_KINETIC / VERTEX_GAUSS / ENERGY), got '{other}'"
            )),
        }
    }

    /// Consume a numeric token (integer or float) and return it as
    /// `f64`. The expect_usize helper rejects fractional inputs; BETA
    /// at GIBBS_SAMPLE accepts any non-negative real.
    #[cfg(feature = "gauge")]
    fn expect_f64(&mut self) -> Result<f64, String> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::Word(w)) => w
                .parse::<f64>()
                .map_err(|_| format!("Expected number, got '{w}'")),
            other => Err(format!("Expected number, got {other:?}")),
        }
    }

    /// Parse an `E_FIELD` declaration. Closes the parser half of the
    /// E_FIELD verb shipped at TDD-HAL-IV.7.
    ///
    /// Grammar:
    ///
    /// ```ebnf
    /// e_field_stmt = "E_FIELD" ident "ON" "GAUGE_FIELD" ident
    ///                "INIT" e_field_init ";" ;
    /// e_field_init = "ZERO"
    ///              | "MAXWELL_BOLTZMANN" "BETA" number [ "SEED" integer ]
    ///              | "FROM" ident ;
    /// ```
    ///
    /// Group erasure (Bee's locked decision IV-B): the parser stays
    /// group-agnostic; the executor matches on the U field's
    /// `handle.group()` and dispatches to the SU(2) `SU2EField` kernel.
    /// Future U(1)/SU(3) E-fields will land as sibling structs with
    /// their own executor arms; the grammar does not change.
    #[cfg(feature = "gauge")]
    fn parse_e_field(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("ON")?;
        self.expect_keyword("GAUGE_FIELD")?;
        let source_gauge_field = self.expect_word()?;
        self.expect_keyword("INIT")?;
        let (init, seed) = self.parse_e_field_init()?;
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Statement::EField {
            name,
            source_gauge_field,
            init,
            seed,
        })
    }

    /// Parse an `e_field_init` per the IV.7 EBNF.
    #[cfg(feature = "gauge")]
    fn parse_e_field_init(
        &mut self,
    ) -> Result<(crate::gauge::EFieldInit, Option<u64>), String> {
        let head = self.expect_word()?;
        match head.to_ascii_uppercase().as_str() {
            "ZERO" => Ok((crate::gauge::EFieldInit::Zero, None)),
            "MAXWELL_BOLTZMANN" => {
                self.expect_keyword("BETA")?;
                let beta = self.expect_f64()?;
                let seed = if self.is_keyword("SEED") {
                    self.advance();
                    Some(self.expect_usize()? as u64)
                } else {
                    None
                };
                Ok((
                    crate::gauge::EFieldInit::MaxwellBoltzmann { beta },
                    seed,
                ))
            }
            "FROM" => {
                let src = self.expect_word()?;
                Ok((crate::gauge::EFieldInit::FromField(src), None))
            }
            other => Err(format!(
                "Expected E_FIELD init clause (ZERO / MAXWELL_BOLTZMANN BETA β / FROM ident), got '{other}'"
            )),
        }
    }

    /// Parse a `SYMPLECTIC_FLOW` statement. Closes the parser half of
    /// TDD-HAL-IV.7.
    ///
    /// Grammar:
    ///
    /// ```ebnf
    /// sympflow_stmt =
    ///   "SYMPLECTIC_FLOW" ident "FROM" "(" "U" "=" ident "," "E" "=" ident ")"
    ///   "BETA" number "DT" number "N_STEPS" integer
    ///   [ "PROJECT_GAUSS" project_gauss_clause ]
    ///   [ "MEASURE_EVERY" integer ]
    ///   [ "MEASURE" "(" observable_list ")" ]
    ///   [ "SEED" integer ]
    ///   ";" ;
    /// ```
    ///
    /// `BETA`/`DT`/`N_STEPS` are mandatory; the rest are optional and
    /// may appear in any order after `N_STEPS` (Halcyon mock-friendly).
    /// `PROJECT_GAUSS` defaults to `None` (no projection) when the
    /// clause is omitted — but production callers should pass
    /// `PROJECT_GAUSS TRUE` (cfg defaults) explicitly per IV-D.
    #[cfg(feature = "gauge")]
    fn parse_symplectic_flow(&mut self) -> Result<Statement, String> {
        let field = self.expect_word()?;
        self.expect_keyword("FROM")?;
        self.expect(Token::LParen)?;
        // U=<ident>, E=<ident>. The `field` ident on the LHS is the
        // SYMPLECTIC_FLOW output name; the U= and E= bindings inside
        // the FROM clause name the two state fields the flow reads +
        // mutates. Matches the Halcyon mock shape.
        self.expect_keyword("U")?;
        self.expect(Token::Eq)?;
        let _u_binding = self.expect_word()?;
        self.expect(Token::Comma)?;
        self.expect_keyword("E")?;
        self.expect(Token::Eq)?;
        let e_field = self.expect_word()?;
        self.expect(Token::RParen)?;

        self.expect_keyword("BETA")?;
        let beta = self.expect_f64()?;
        self.expect_keyword("DT")?;
        let dt = self.expect_f64()?;
        self.expect_keyword("N_STEPS")?;
        let n_steps = self.expect_usize()?;

        let mut project_gauss: Option<crate::gauge::ProjectGaussConfig> = None;
        let mut measure_every: usize = 1;
        let mut measure: Vec<crate::gauge::ObservableId> = Vec::new();
        let mut seed: Option<u64> = None;

        loop {
            if self.is_keyword("PROJECT_GAUSS") {
                self.advance();
                project_gauss = self.parse_project_gauss_clause()?;
            } else if self.is_keyword("MEASURE_EVERY") {
                self.advance();
                measure_every = self.expect_usize()?;
            } else if self.is_keyword("MEASURE") {
                self.advance();
                self.expect(Token::LParen)?;
                measure = self.parse_observable_list()?;
                self.expect(Token::RParen)?;
            } else if self.is_keyword("SEED") {
                self.advance();
                seed = Some(self.expect_usize()? as u64);
            } else {
                break;
            }
        }
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Statement::SymplecticFlow {
            field,
            e_field,
            beta,
            dt,
            n_steps,
            project_gauss,
            measure_every,
            measure,
            seed,
        })
    }

    /// Parse a `LOOP name ON lattice FACE n;` or
    /// `LOOP name ON lattice EDGES (v0, v1, …);` declaration.
    ///
    /// Halcyon Part VI deliverable #2 companion verb. Registers a
    /// named loop against a lattice; `LOOP_TRANSPORT`'s `ALONG_LOOP`
    /// clause resolves names through this registry. FACE-form is
    /// always closed; EDGES-form may be open (the `LoopNotClosed`
    /// rejection in `LOOP_TRANSPORT` is what surfaces an open path
    /// per the gate doc §SHAM audit-story flag).
    #[cfg(feature = "gauge")]
    fn parse_loop_decl(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("ON")?;
        let lattice = self.expect_word()?;
        let body = if self.is_keyword("FACE") {
            self.advance();
            let n = self.expect_usize()?;
            LoopBody::Face(n)
        } else if self.is_keyword("EDGES") {
            self.advance();
            self.expect(Token::LParen)?;
            let mut vs: Vec<usize> = Vec::new();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !vs.is_empty() {
                    self.expect(Token::Comma)?;
                }
                vs.push(self.expect_usize()?);
            }
            self.expect(Token::RParen)?;
            LoopBody::Edges(vs)
        } else {
            return Err(format!(
                "Expected FACE or EDGES in LOOP declaration, got {:?}",
                self.peek()
            ));
        };
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Statement::LoopDecl {
            name,
            lattice,
            body,
        })
    }

    /// Parse a `LOOP_TRANSPORT` statement per
    /// `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3` §4.4 (Zenodo DOI
    /// 10.5281/zenodo.20785681). Halcyon Part VI deliverable #2.
    ///
    /// Grammar (frozen at v3.1.3 deposit; spec name is
    /// SAMPLE_TRANSPORT, implementation name LOOP_TRANSPORT per
    /// rename agreement):
    ///
    /// ```ebnf
    /// loop_transport_stmt =
    ///   "LOOP_TRANSPORT" ident
    ///     "ALONG_LOOP" ident
    ///     "CONTROL_MANIFOLD" "(" "Q" "," "BETA_WILSON" ")"
    ///     "ADIABATIC" ("TRUE" | "FALSE")
    ///     "RAMP_RATE_Q" number "RAMP_RATE_BETA_W" number
    ///     "DRIVE_OMEGA" number "DRIVE_F0" number
    ///     "N_DISCRETIZATION" integer
    ///     "PIN_LAMBDA_Q" number "PIN_LAMBDA_BETA_W" number
    ///     "EPS_Q" number "EPS_BETA_W" number
    ///     "ALPHA_HALCYON" number "TAU_0" number "BETA_TAU" number
    ///     "MU_BASELINE" number "K_SPRING" number "C_DAMP" number
    ///     [ "BETA_WILSON_START" number ]
    ///     "SEEDS" "[" integer ".." integer "]"
    ///     compute_clause+
    ///     [ "SHAM" "{" sham_flag* "}" ]
    ///     "RETURN" return_field ("," return_field)*
    ///     ";" ;
    /// ```
    ///
    /// `BETA_WILSON_START` is an additive knob over the frozen grammar
    /// — v3.1.3 §2 says "BETA_WILSON ∈ [2.5, 3.0]" without specifying
    /// the START coordinate (the ramp gives slope only). The parser
    /// validates the (start, start + ramp · T_segment) endpoints
    /// against the regime; missing → defaults to the regime midpoint
    /// 2.75.
    #[cfg(feature = "gauge")]
    fn parse_loop_transport(&mut self) -> Result<Statement, String> {
        let lattice = self.expect_word()?;
        self.expect_keyword("ALONG_LOOP")?;
        let loop_id = self.expect_word()?;
        self.expect_keyword("CONTROL_MANIFOLD")?;
        self.expect(Token::LParen)?;
        // v3.1.3 freezes (Q, BETA_WILSON).
        let q_kw = self.expect_word()?;
        if !q_kw.eq_ignore_ascii_case("Q") {
            return Err(format!(
                "Expected CONTROL_MANIFOLD (Q, BETA_WILSON), got first axis '{q_kw}'"
            ));
        }
        self.expect(Token::Comma)?;
        let bw_kw = self.expect_word()?;
        if !bw_kw.eq_ignore_ascii_case("BETA_WILSON") {
            return Err(format!(
                "Expected CONTROL_MANIFOLD (Q, BETA_WILSON), got second axis '{bw_kw}'"
            ));
        }
        self.expect(Token::RParen)?;
        let control_manifold = ControlManifoldSpec::QBetaWilson;

        // Remaining clauses are order-friendly (matches
        // SYMPLECTIC_FLOW optional-clause loop). v3.1.3 §4.4 shows a
        // canonical order; we accept any order and validate
        // completeness at the end.
        let mut adiabatic: Option<bool> = None;
        let mut ramp_rate_q: Option<f64> = None;
        let mut ramp_rate_beta_w: Option<f64> = None;
        let mut drive_omega: Option<f64> = None;
        let mut drive_f0: Option<f64> = None;
        let mut n_discretization: Option<usize> = None;
        let mut pin_lambda_q: Option<f64> = None;
        let mut pin_lambda_beta_w: Option<f64> = None;
        let mut eps_q: Option<f64> = None;
        let mut eps_beta_w: Option<f64> = None;
        let mut alpha_halcyon: Option<f64> = None;
        let mut tau_0: Option<f64> = None;
        let mut beta_tau: Option<f64> = None;
        let mut mu_baseline: Option<f64> = None;
        let mut k_spring: Option<f64> = None;
        let mut c_damp: Option<f64> = None;
        let mut beta_wilson_start: Option<f64> = None;
        let mut seeds: Option<SeedRange> = None;
        let mut compute: Vec<LoopTransportOutputId> = Vec::new();
        let mut sham: Option<ShamBlock> = None;
        let mut return_fields: Vec<LoopTransportReturnId> = Vec::new();

        loop {
            if self.is_keyword("ADIABATIC") {
                self.advance();
                adiabatic = Some(self.expect_bool_word()?);
            } else if self.is_keyword("RAMP_RATE_Q") {
                self.advance();
                ramp_rate_q = Some(self.expect_f64()?);
            } else if self.is_keyword("RAMP_RATE_BETA_W") {
                self.advance();
                ramp_rate_beta_w = Some(self.expect_f64()?);
            } else if self.is_keyword("DRIVE_OMEGA") {
                self.advance();
                drive_omega = Some(self.expect_f64()?);
            } else if self.is_keyword("DRIVE_F0") {
                self.advance();
                drive_f0 = Some(self.expect_f64()?);
            } else if self.is_keyword("N_DISCRETIZATION") {
                self.advance();
                n_discretization = Some(self.expect_usize()?);
            } else if self.is_keyword("PIN_LAMBDA_Q") {
                self.advance();
                pin_lambda_q = Some(self.expect_f64()?);
            } else if self.is_keyword("PIN_LAMBDA_BETA_W") {
                self.advance();
                pin_lambda_beta_w = Some(self.expect_f64()?);
            } else if self.is_keyword("EPS_Q") {
                self.advance();
                eps_q = Some(self.expect_f64()?);
            } else if self.is_keyword("EPS_BETA_W") {
                self.advance();
                eps_beta_w = Some(self.expect_f64()?);
            } else if self.is_keyword("ALPHA_HALCYON") {
                self.advance();
                alpha_halcyon = Some(self.expect_f64()?);
            } else if self.is_keyword("TAU_0") {
                self.advance();
                tau_0 = Some(self.expect_f64()?);
            } else if self.is_keyword("BETA_TAU") {
                self.advance();
                beta_tau = Some(self.expect_f64()?);
            } else if self.is_keyword("MU_BASELINE") {
                self.advance();
                mu_baseline = Some(self.expect_f64()?);
            } else if self.is_keyword("K_SPRING") {
                self.advance();
                k_spring = Some(self.expect_f64()?);
            } else if self.is_keyword("C_DAMP") {
                self.advance();
                c_damp = Some(self.expect_f64()?);
            } else if self.is_keyword("BETA_WILSON_START") {
                self.advance();
                beta_wilson_start = Some(self.expect_f64()?);
            } else if self.is_keyword("SEEDS") {
                self.advance();
                seeds = Some(self.parse_seed_bracket()?);
            } else if self.is_keyword("COMPUTE") {
                self.advance();
                compute.push(self.parse_loop_transport_output_id()?);
            } else if self.is_keyword("SHAM") {
                self.advance();
                sham = Some(self.parse_sham_block()?);
            } else if self.is_keyword("RETURN") {
                self.advance();
                return_fields = self.parse_loop_transport_return_list()?;
            } else {
                break;
            }
        }
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }

        // Required-clause check. Each missing required clause names
        // itself so the rejection test (which asserts the parser
        // error contains "N_DISCRETIZATION") matches deterministically.
        let stmt = Statement::LoopTransport {
            lattice,
            loop_id,
            control_manifold,
            adiabatic: adiabatic
                .ok_or_else(|| "LOOP_TRANSPORT missing ADIABATIC".to_string())?,
            ramp_rate_q: ramp_rate_q
                .ok_or_else(|| "LOOP_TRANSPORT missing RAMP_RATE_Q".to_string())?,
            ramp_rate_beta_w: ramp_rate_beta_w
                .ok_or_else(|| "LOOP_TRANSPORT missing RAMP_RATE_BETA_W".to_string())?,
            drive_omega: drive_omega
                .ok_or_else(|| "LOOP_TRANSPORT missing DRIVE_OMEGA".to_string())?,
            drive_f0: drive_f0
                .ok_or_else(|| "LOOP_TRANSPORT missing DRIVE_F0".to_string())?,
            n_discretization: n_discretization
                .ok_or_else(|| "LOOP_TRANSPORT missing N_DISCRETIZATION".to_string())?,
            pin_lambda_q: pin_lambda_q
                .ok_or_else(|| "LOOP_TRANSPORT missing PIN_LAMBDA_Q".to_string())?,
            pin_lambda_beta_w: pin_lambda_beta_w
                .ok_or_else(|| "LOOP_TRANSPORT missing PIN_LAMBDA_BETA_W".to_string())?,
            eps_q: eps_q.ok_or_else(|| "LOOP_TRANSPORT missing EPS_Q".to_string())?,
            eps_beta_w: eps_beta_w
                .ok_or_else(|| "LOOP_TRANSPORT missing EPS_BETA_W".to_string())?,
            alpha_halcyon: alpha_halcyon
                .ok_or_else(|| "LOOP_TRANSPORT missing ALPHA_HALCYON".to_string())?,
            tau_0: tau_0.ok_or_else(|| "LOOP_TRANSPORT missing TAU_0".to_string())?,
            beta_tau: beta_tau
                .ok_or_else(|| "LOOP_TRANSPORT missing BETA_TAU".to_string())?,
            mu_baseline: mu_baseline
                .ok_or_else(|| "LOOP_TRANSPORT missing MU_BASELINE".to_string())?,
            k_spring: k_spring
                .ok_or_else(|| "LOOP_TRANSPORT missing K_SPRING".to_string())?,
            c_damp: c_damp.ok_or_else(|| "LOOP_TRANSPORT missing C_DAMP".to_string())?,
            seeds: seeds.ok_or_else(|| "LOOP_TRANSPORT missing SEEDS".to_string())?,
            compute,
            return_fields,
            sham,
            beta_wilson_start: beta_wilson_start.unwrap_or(2.75_f64),
        };

        // β_W validation at parse time per the gate doc's "audit-story
        // flag" stance: the verb either lands inside the v3.1.3 §2
        // validated regime [2.5, 3.0] before the executor starts, or
        // it doesn't run.
        //
        // VI.6b Fix #5 — amplitude-based bound instead of open-chain
        // endpoint extrapolation. The loop CLOSES in the (Q, β_W)
        // control manifold; the relevant bound is max |β_W − β_start|
        // over γ, not the open-chain endpoint
        // β_start + ramp · T_segment (which scales with α and forces
        // α=1000 to reject at 12.5 even though the closed-loop max
        // never leaves the regime).
        //
        // The canonical γ_unit's half-amplitude on the BETA_WILSON
        // axis is
        //
        //     amp = |RAMP_RATE_BETA_W| · TAU_0 / 4
        //
        // — α-independent and N_DISCRETIZATION-independent. At
        // canonical (ramp=0.01, tau_0=1) this is 0.0025; the regime
        // [2.5, 3.0] easily contains β_start ± amp for any
        // β_start ∈ [2.5, 3.0]. The α-independence is the whole point
        // of the fix: v3.1.3 §3.6 requires BOTH α=1 AND α=1000
        // calibrations to pass the same parser-stage validation, and
        // the gold-fixture canonical (N=10000, α=1) stays bit-
        // identical because the verb body never read the validator's
        // returned `beta_end` — only `beta_start`.
        //
        // BETA_WILSON_START is an additive knob (v3.1.3 §2 freezes the
        // regime without specifying the launch coordinate; the
        // rejection battery uses this to push β_W in or out of
        // regime). Missing → defaults to the regime midpoint 2.75.
        let beta_start = beta_wilson_start.unwrap_or(2.75_f64);
        let (ramp_bw, tau) = match &stmt {
            Statement::LoopTransport {
                ramp_rate_beta_w,
                tau_0,
                ..
            } => (*ramp_rate_beta_w, *tau_0),
            _ => unreachable!("just constructed Statement::LoopTransport"),
        };
        let beta_w_amplitude = ramp_bw.abs() * tau / 4.0_f64;
        let max_beta_w_reached = beta_start + beta_w_amplitude;
        let range_lo = 2.5_f64;
        let range_hi = 3.0_f64;
        // Per Locked spec: reject if max_beta_w_reached > 3.0 OR
        // beta_start < 2.5. β_start above the regime is rejected
        // implicitly because max ≥ β_start trips the upper-bound rule.
        // The pin penalties (PIN_LAMBDA_BETA_W) keep the actual β
        // tightly clamped to the ramp reference, so the canonical
        // γ_unit's negative excursion is dominated by f64 round-off
        // and not by the open-chain ramp shape — we don't gate on the
        // lower extremum of the closed-loop amplitude.
        if beta_start < range_lo {
            return Err(format!(
                "BetaWilsonOutOfValidatedRegime: BETA_WILSON = {beta_start} outside validated regime [{:.1}, {:.1}]",
                range_lo, range_hi
            ));
        }
        if max_beta_w_reached > range_hi {
            return Err(format!(
                "BetaWilsonOutOfValidatedRegime: BETA_WILSON = {max_beta_w_reached} outside validated regime [{:.1}, {:.1}]",
                range_lo, range_hi
            ));
        }

        Ok(stmt)
    }

    /// Parse `[lo..hi]` (inclusive both ends) for LOOP_TRANSPORT SEEDS.
    fn parse_seed_bracket(&mut self) -> Result<SeedRange, String> {
        self.expect(Token::LBracket)?;
        let lo = self.expect_usize()? as u64;
        self.expect(Token::DotDot)?;
        let hi = self.expect_usize()? as u64;
        self.expect(Token::RBracket)?;
        Ok(SeedRange { lo, hi })
    }

    /// Parse a single COMPUTE clause keyword.
    fn parse_loop_transport_output_id(&mut self) -> Result<LoopTransportOutputId, String> {
        let kw = self.expect_word()?;
        match kw.to_ascii_uppercase().as_str() {
            "HOLONOMY_FORWARD" => Ok(LoopTransportOutputId::HolonomyForward),
            "HOLONOMY_REVERSED" => Ok(LoopTransportOutputId::HolonomyReversed),
            "TRACKING_ERROR_TRACE_Q" => Ok(LoopTransportOutputId::TrackingErrorTraceQ),
            "TRACKING_ERROR_TRACE_BETA_W" => Ok(LoopTransportOutputId::TrackingErrorTraceBetaW),
            "ADIABATICITY_CHECK" => Ok(LoopTransportOutputId::AdiabaticityCheck),
            other => Err(format!("LOOP_TRANSPORT COMPUTE: unknown output id '{other}'")),
        }
    }

    /// Parse one RETURN identifier (case-insensitive).
    fn parse_loop_transport_return_id(&mut self) -> Result<LoopTransportReturnId, String> {
        let kw = self.expect_word()?;
        match kw.to_ascii_uppercase().as_str() {
            "H_FORWARD" => Ok(LoopTransportReturnId::HForward),
            "H_REVERSED" => Ok(LoopTransportReturnId::HReversed),
            "SIGMA_H_BLOCKED" => Ok(LoopTransportReturnId::SigmaHBlocked),
            "PER_SEED_H_FORWARD" => Ok(LoopTransportReturnId::PerSeedHForward),
            "PER_SEED_H_REVERSED" => Ok(LoopTransportReturnId::PerSeedHReversed),
            "TRACKING_ERROR_MAX_Q" => Ok(LoopTransportReturnId::TrackingErrorMaxQ),
            "TRACKING_ERROR_MAX_BETA_W" => Ok(LoopTransportReturnId::TrackingErrorMaxBetaW),
            "ADIABATICITY_CHECK" => Ok(LoopTransportReturnId::AdiabaticityCheck),
            other => Err(format!("LOOP_TRANSPORT RETURN: unknown field '{other}'")),
        }
    }

    /// Parse a comma-separated RETURN list.
    fn parse_loop_transport_return_list(&mut self) -> Result<Vec<LoopTransportReturnId>, String> {
        let mut out = Vec::new();
        loop {
            out.push(self.parse_loop_transport_return_id()?);
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// Parse a `SHAM { flag [= value] [, flag …] }` block. Empty
    /// blocks are allowed (the VI.2 contract: empty SHAM parses +
    /// executes; non-empty rejects at executor entry with
    /// UnrecognizedShamFlag).
    fn parse_sham_block(&mut self) -> Result<ShamBlock, String> {
        self.expect(Token::LBrace)?;
        let mut flags: Vec<(String, ShamArg)> = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::RBrace)) {
                break;
            }
            if !flags.is_empty() {
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
                if matches!(self.peek(), Some(Token::RBrace)) {
                    break;
                }
            }
            let name = self.expect_word()?;
            let arg = if matches!(self.peek(), Some(Token::Eq)) {
                self.advance();
                match self.advance() {
                    Some(Token::Number(n)) => ShamArg::Number(n),
                    Some(Token::Str(s)) => ShamArg::Text(s),
                    Some(Token::Word(w)) => {
                        if w.eq_ignore_ascii_case("TRUE") {
                            ShamArg::Bool(true)
                        } else if w.eq_ignore_ascii_case("FALSE") {
                            ShamArg::Bool(false)
                        } else {
                            ShamArg::Text(w)
                        }
                    }
                    other => return Err(format!("Expected SHAM flag value, got {other:?}")),
                }
            } else {
                ShamArg::Bool(true)
            };
            flags.push((name, arg));
        }
        self.expect(Token::RBrace)?;
        Ok(ShamBlock { flags })
    }

    /// Parse a TRUE/FALSE keyword as a bool (case-insensitive).
    fn expect_bool_word(&mut self) -> Result<bool, String> {
        match self.advance() {
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("TRUE") => Ok(true),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("FALSE") => Ok(false),
            other => Err(format!("Expected TRUE or FALSE, got {other:?}")),
        }
    }

    /// Parse a `project_gauss_clause` per the IV.7 EBNF.
    ///
    /// ```ebnf
    /// project_gauss_clause = "TRUE" | "FALSE"
    ///                      | "{" project_gauss_kv ("," project_gauss_kv)* "}" ;
    /// project_gauss_kv     = ("tikhonov" | "cg_tol" | "cg_max_iter")
    ///                        ":" number_or_int ;
    /// ```
    ///
    /// `TRUE` returns `Some(ProjectGaussConfig::default())`; `FALSE`
    /// returns `None`; struct-literal sugar returns a config with the
    /// named fields overridden against `Default` (per IV-A: tikhonov
    /// 1e-14, cg_tol 1e-10, cg_max_iter 200). Unspecified fields keep
    /// their defaults — the spec-shape 1e-12 tikhonov is reachable via
    /// `PROJECT_GAUSS { tikhonov: 1e-12 }`.
    ///
    /// The lexer emits `LBrace` / `RBrace` tokens; the keys
    /// (`tikhonov` / `cg_tol` / `cg_max_iter`) are words followed by a
    /// `Colon` and a numeric literal. Matches how Halcyon's mock
    /// generates the GQL string.
    #[cfg(feature = "gauge")]
    fn parse_project_gauss_clause(
        &mut self,
    ) -> Result<Option<crate::gauge::ProjectGaussConfig>, String> {
        // Peek for TRUE / FALSE / struct-literal opening token.
        if self.is_keyword("TRUE") {
            self.advance();
            return Ok(Some(crate::gauge::ProjectGaussConfig::default()));
        }
        if self.is_keyword("FALSE") {
            self.advance();
            return Ok(None);
        }
        // Struct-literal form: `{ key: value, … }`.
        match self.peek() {
            Some(Token::LBrace) => {
                self.advance();
            }
            other => {
                return Err(format!(
                    "Expected PROJECT_GAUSS clause (TRUE / FALSE / {{ … }}), got {other:?}"
                ))
            }
        }
        let mut cfg = crate::gauge::ProjectGaussConfig::default();
        loop {
            let key = self.expect_word()?;
            self.expect(Token::Colon)?;
            let val = self.expect_f64()?;
            match key.as_str() {
                "tikhonov" => cfg.tikhonov = val,
                "cg_tol" => cfg.cg_tol = val,
                "cg_max_iter" => {
                    if val < 0.0 || val.fract() != 0.0 {
                        return Err(format!(
                            "PROJECT_GAUSS cg_max_iter must be a non-negative integer, got {val}"
                        ));
                    }
                    cfg.cg_max_iter = val as usize;
                }
                other => {
                    return Err(format!(
                        "Unknown PROJECT_GAUSS field '{other}' (allowed: tikhonov / cg_tol / cg_max_iter)"
                    ))
                }
            }
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(Token::RBrace)?;
        Ok(Some(cfg))
    }

    /// TDD-HAL-III.6: try to parse a gauge-substrate SELECT projection
    /// (`SELECT PLAQUETTE OF U;`, `SELECT MEAN(PLAQUETTE OF U);`,
    /// `SELECT SUM(PLAQUETTE OF U);`, `SELECT Q_SURROGATE OF U;`)
    /// before falling through to the bundle-shaped `parse_sql_select`.
    /// Returns `Ok(None)` when the next token is not a gauge-prefixed
    /// keyword (so the caller falls through to the classic SELECT
    /// path); `Ok(Some(stmt))` on a successful gauge SELECT; `Err` on
    /// a malformed gauge SELECT.
    ///
    /// Lookahead is one token deep — we peek `PLAQUETTE` /
    /// `Q_SURROGATE` directly, and one token past `MEAN` / `SUM` to
    /// distinguish `MEAN(field)` (classic agg) from
    /// `MEAN(PLAQUETTE OF U)` (gauge projection). The peek is
    /// non-consuming so the fall-through path sees the full token
    /// stream unchanged.
    #[cfg(feature = "gauge")]
    fn try_parse_gauge_select(&mut self) -> Result<Option<Statement>, String> {
        let head = match self.peek() {
            Some(Token::Word(w)) => w.to_ascii_uppercase(),
            _ => return Ok(None),
        };
        let red = match head.as_str() {
            "PLAQUETTE" => {
                self.advance();
                self.expect_keyword("OF")?;
                let field = self.expect_word()?;
                if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.advance();
                }
                return Ok(Some(Statement::SelectPlaquette {
                    field,
                    reduction: crate::gauge::PlaquetteReduction::PerFace,
                }));
            }
            "Q_SURROGATE" => {
                self.advance();
                self.expect_keyword("OF")?;
                let field = self.expect_word()?;
                if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.advance();
                }
                return Ok(Some(Statement::SelectQSurrogate { field }));
            }
            // TDD-HAL-IV.7: `SELECT H_TOTAL OF (U, E);` — joint
            // observable on the (U, E) pair. Lowers to the
            // Part-IV-aware E-aware Hamiltonian computation at the
            // executor.
            "H_TOTAL" => {
                self.advance();
                self.expect_keyword("OF")?;
                self.expect(Token::LParen)?;
                let u = self.expect_word()?;
                self.expect(Token::Comma)?;
                let e = self.expect_word()?;
                self.expect(Token::RParen)?;
                if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.advance();
                }
                return Ok(Some(Statement::SelectHTotal {
                    gauge_field: u,
                    e_field: e,
                }));
            }
            // TDD-HAL-IV.7: `SELECT GAUSS_RESIDUAL_MAX OF (U, E);` —
            // joint observable on the (U, E) pair. Default reduction
            // is `Covariant`; explicit `Flat` reduction is reserved
            // for future use (debug-only / P1).
            "GAUSS_RESIDUAL_MAX" => {
                self.advance();
                self.expect_keyword("OF")?;
                self.expect(Token::LParen)?;
                let u = self.expect_word()?;
                self.expect(Token::Comma)?;
                let e = self.expect_word()?;
                self.expect(Token::RParen)?;
                if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.advance();
                }
                return Ok(Some(Statement::SelectGaussResidualMax {
                    gauge_field: u,
                    e_field: e,
                    reduction: crate::gauge::GaussReduction::Covariant,
                }));
            }
            "MEAN" => crate::gauge::PlaquetteReduction::Mean,
            "SUM" => crate::gauge::PlaquetteReduction::Sum,
            _ => return Ok(None),
        };
        // Two-token lookahead through MEAN(/SUM( … so we can distinguish
        // a gauge projection (PLAQUETTE inside the parens) from a
        // classic aggregate (field-name inside the parens).
        let saved_pos = self.pos;
        self.advance(); // consume MEAN/SUM
        if !matches!(self.peek(), Some(Token::LParen)) {
            self.pos = saved_pos;
            return Ok(None);
        }
        self.advance(); // consume LParen
        let is_gauge = matches!(
            self.peek(),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("PLAQUETTE")
        );
        if !is_gauge {
            // Classic aggregate — rewind and let parse_sql_select see
            // it.
            self.pos = saved_pos;
            return Ok(None);
        }
        self.advance(); // consume PLAQUETTE
        self.expect_keyword("OF")?;
        let field = self.expect_word()?;
        self.expect(Token::RParen)?;
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
        Ok(Some(Statement::SelectPlaquette {
            field,
            reduction: red,
        }))
    }

    // ── GQL: INTEGRATE (aggregation) ──

    fn parse_integrate(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let mut over = None;
        let mut measures = Vec::new();

        if self.is_keyword("OVER") {
            self.advance();
            over = Some(self.expect_word()?);
        }

        if self.is_keyword("MEASURE") {
            self.advance();
            loop {
                let func_name = self.expect_word()?;
                let func = match func_name.to_ascii_uppercase().as_str() {
                    "COUNT" => AggFunc::Count,
                    "SUM" => AggFunc::Sum,
                    "AVG" => AggFunc::Avg,
                    "MIN" => AggFunc::Min,
                    "MAX" => AggFunc::Max,
                    _ => return Err(format!("Unknown aggregate: {func_name}")),
                };
                self.expect(Token::LParen)?;
                let field = if matches!(self.peek(), Some(Token::Star)) {
                    self.advance();
                    "*".to_string()
                } else {
                    self.expect_word()?
                };
                self.expect(Token::RParen)?;

                let alias = if self.is_keyword("AS") {
                    self.advance();
                    Some(self.expect_word()?)
                } else {
                    None
                };

                measures.push(MeasureSpec { func, field, alias });

                if !matches!(self.peek(), Some(Token::Comma)) {
                    break;
                }
                self.advance(); // consume comma
            }
        }

        Ok(Statement::Integrate {
            bundle: name,
            over,
            measures,
        })
    }

    // ── GQL: PULLBACK (join) ──

    fn parse_pullback(&mut self) -> Result<Statement, String> {
        let left = self.expect_word()?;
        self.expect_keyword("ALONG")?;
        let along = self.expect_word()?;
        self.expect_keyword("ONTO")?;
        let right = self.expect_word()?;

        let right_field = if self.is_keyword("ALONG") {
            self.advance();
            Some(self.expect_word()?)
        } else {
            None
        };

        let preserve_left = if self.is_keyword("PRESERVE") {
            self.advance();
            self.expect_keyword("LEFT")?;
            true
        } else {
            false
        };

        Ok(Statement::Pullback {
            left,
            along,
            right,
            right_field,
            preserve_left,
        })
    }

    // ── GQL: CURVATURE / SPECTRAL / CONSISTENCY ──

    fn parse_curvature(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let mut fields = Vec::new();
        let mut by_field = None;

        if self.is_keyword("ON") {
            self.advance();
            loop {
                fields.push(self.expect_word()?);
                if !matches!(self.peek(), Some(Token::Comma)) {
                    break;
                }
                self.advance();
            }
        }

        if self.is_keyword("BY") {
            self.advance();
            by_field = Some(self.expect_word()?);
        }

        Ok(Statement::Curvature {
            bundle: name,
            fields,
            by_field,
        })
    }

    fn parse_spectral(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        // Check for ON FIBER variant: SPECTRAL bundle ON FIBER (f1, f2) MODES k
        if self.is_keyword("ON") {
            self.advance(); // ON
            self.expect_word()?; // FIBER
            self.expect(Token::LParen)?;
            let fiber_fields = self.parse_inner_word_list()?;
            self.expect_word()?; // MODES
            let modes = self.expect_usize()?;
            return Ok(Statement::SpectralFiber { bundle: name, fiber_fields, modes });
        }
        let full = if self.is_keyword("FULL") {
            self.advance();
            true
        } else {
            false
        };
        Ok(Statement::Spectral { bundle: name, full })
    }

    fn parse_consistency(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let repair = if self.is_keyword("REPAIR") {
            self.advance();
            true
        } else {
            false
        };
        Ok(Statement::Consistency {
            bundle: name,
            repair,
        })
    }

    fn parse_betti(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        Ok(Statement::Betti { bundle: name })
    }

    fn parse_entropy(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        Ok(Statement::Entropy { bundle: name })
    }

    /// Sprint H: `PROJECT INVARIANT (e1, e2, ...) FROM <bundle> [WHERE <cond>]`
    ///
    /// The grammar admits ONLY whitelisted invariant operations and arithmetic
    /// of those (with constants). A query that compiles is structurally
    /// guaranteed to never call a `decrypt_*` function — the evaluator's
    /// dispatch table contains no path that reads ciphertext.
    fn parse_project_invariant(&mut self) -> Result<Statement, String> {
        // Already consumed `PROJECT` in dispatch.
        self.expect_keyword("INVARIANT")?;
        self.expect(Token::LParen)?;

        let mut expressions: Vec<(String, InvariantExpr)> = Vec::new();
        loop {
            let start_pos = self.pos;
            let expr = self.parse_invariant_expr()?;
            // Reconstruct a canonical label from the consumed tokens.
            // Crude but stable: just print what was parsed.
            let label = invariant_label(&expr);
            expressions.push((label, expr));
            let _ = start_pos;

            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(Token::RParen)?;

        self.expect_keyword("FROM")?;
        let bundle = self.expect_word()?;

        let where_clause = if self.is_keyword("WHERE") {
            self.advance();
            Some(self.parse_filter_conditions()?)
        } else {
            None
        };

        Ok(Statement::ProjectInvariant {
            bundle,
            expressions,
            where_clause,
        })
    }

    /// Pratt-style parser for invariant expressions: term (+ term | * term)*.
    /// Keeps things simple — no operator precedence beyond left-to-right, since
    /// the surface is small (just + and *) and parens disambiguate.
    fn parse_invariant_expr(&mut self) -> Result<InvariantExpr, String> {
        let mut lhs = self.parse_invariant_term()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.advance();
                    let rhs = self.parse_invariant_term()?;
                    lhs = InvariantExpr::Add(Box::new(lhs), Box::new(rhs));
                }
                Some(Token::Star) => {
                    self.advance();
                    let rhs = self.parse_invariant_term()?;
                    lhs = InvariantExpr::Mul(Box::new(lhs), Box::new(rhs));
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_invariant_term(&mut self) -> Result<InvariantExpr, String> {
        // Constant: a literal Number.
        if let Some(Token::Number(n)) = self.peek() {
            let v = *n;
            self.advance();
            return Ok(InvariantExpr::Const(v));
        }
        // Parenthesized sub-expression.
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let inner = self.parse_invariant_expr()?;
            self.expect(Token::RParen)?;
            return Ok(inner);
        }
        // Word: must be a whitelisted op.
        let word = self.expect_word()?;
        let op = match word.to_ascii_lowercase().as_str() {
            "curvature" => InvariantOp::Curvature,
            "confidence" => InvariantOp::Confidence,
            "capacity" => {
                // Required parameter: capacity(tau). Tau is the tolerance
                // scalar for the Davis Law C = τ/K.
                self.expect(Token::LParen)?;
                let tau = match self.advance() {
                    Some(Token::Number(n)) => n,
                    other => {
                        return Err(format!(
                            "capacity requires a numeric tolerance: capacity(tau). Got {other:?}"
                        ));
                    }
                };
                self.expect(Token::RParen)?;
                InvariantOp::Capacity { tau }
            }
            "spectral_gap" => InvariantOp::SpectralGap,
            "beta_0" => InvariantOp::Beta0,
            "beta_1" => InvariantOp::Beta1,
            "holonomy_avg" => InvariantOp::HolonomyAvg,
            other => {
                return Err(format!(
                    "PROJECT INVARIANT: unknown invariant `{other}`. \
                     Allowed: curvature, confidence, capacity(tau), \
                     spectral_gap, beta_0, beta_1, holonomy_avg. \
                     (entropy currently requires fiber-value access; use the \
                     ENTROPY top-level statement instead.)"
                ));
            }
        };
        Ok(InvariantExpr::Op(op))
    }

    fn parse_free_energy(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("AT")?;
        match self.tokens.get(self.pos) {
            Some(Token::Number(n)) => {
                let tau = *n;
                self.pos += 1;
                Ok(Statement::FreeEnergy { bundle: name, tau })
            }
            other => Err(format!("expected number for tau, got {:?}", other)),
        }
    }

    /// CAPACITY <bundle> [TOLERANCE τ]
    ///
    /// Returns Davis capacity C = τ/K for the bundle. τ defaults to 1.0.
    /// The TOLERANCE keyword mirrors the planned GQL_REFERENCE syntax.
    fn parse_capacity_stmt(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let tau = if self.is_keyword("TOLERANCE") {
            self.advance();
            match self.tokens.get(self.pos) {
                Some(Token::Number(n)) => { let v = *n; self.pos += 1; v }
                other => return Err(format!("CAPACITY: expected τ after TOLERANCE, got {other:?}")),
            }
        } else {
            1.0 // default: C = 1/K (τ=1 makes C dimensionless)
        };
        Ok(Statement::Capacity { bundle, tau })
    }

    /// HORIZON <bundle> [TOLERANCE τ]
    ///
    /// Returns holonomy horizon s_max = τ/(K·ℓ_c).
    /// τ defaults to 1.0; ℓ_c estimated from spectral gap as 1/√λ₁.
    fn parse_horizon_stmt(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let tau = if self.is_keyword("TOLERANCE") {
            self.advance();
            match self.tokens.get(self.pos) {
                Some(Token::Number(n)) => { let v = *n; self.pos += 1; v }
                other => return Err(format!("HORIZON: expected τ after TOLERANCE, got {other:?}")),
            }
        } else {
            1.0
        };
        // Optional LENGTH_SCALE override clause.
        let config = if self.is_keyword("LENGTH_SCALE") {
            self.advance();
            let kind = self.expect_word()?.to_uppercase();
            let estimator = match kind.as_str() {
                "SPECTRAL_GAP" => crate::curvature::LengthScaleEstimator::SpectralGap,
                "WELFORD_RADIUS" => crate::curvature::LengthScaleEstimator::WelfordRadius,
                "FIXED" => {
                    let v = match self.tokens.get(self.pos) {
                        Some(Token::Number(n)) => { let v = *n; self.pos += 1; v }
                        other => return Err(format!(
                            "HORIZON: LENGTH_SCALE FIXED expects a number, got {other:?}"
                        )),
                    };
                    crate::curvature::LengthScaleEstimator::Fixed(v)
                }
                other => return Err(format!(
                    "HORIZON: LENGTH_SCALE kind must be one of \
                     SPECTRAL_GAP | WELFORD_RADIUS | FIXED <n>; got {other}"
                )),
            };
            Some(crate::curvature::HorizonConfig {
                estimator,
                ..crate::curvature::HorizonConfig::default()
            })
        } else {
            None
        };
        Ok(Statement::Horizon { bundle, tau, config })
    }

    /// DEPTH <bundle>
    ///   [K_METRIC <f64>]            — overrides `DepthConfig::k_metric` (default 0.5)
    ///   [K_CONNECTION <f64>]        — overrides `DepthConfig::k_connection` (default 0.1)
    ///   [LAMBDA1_TOPOLOGICAL <f64>] — overrides `DepthConfig::lambda1_topological` (default 0.01)
    ///   [LAMBDA1_CONNECTION <f64>]  — overrides `DepthConfig::lambda1_connection` (default 0.3)
    ///
    /// Returns the encoding depth classification (I–IV) based on K and λ₁.
    /// All four threshold keywords are optional and may appear in any
    /// order; unspecified thresholds use the published defaults from
    /// Theorem 8.14 of the Cognitive Geometry Correspondence.
    fn parse_depth_stmt(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let mut overrides: Option<crate::curvature::DepthConfig> = None;
        loop {
            let kw = if self.is_keyword("K_METRIC") {
                "k_metric"
            } else if self.is_keyword("K_CONNECTION") {
                "k_connection"
            } else if self.is_keyword("LAMBDA1_TOPOLOGICAL") {
                "lambda1_topological"
            } else if self.is_keyword("LAMBDA1_CONNECTION") {
                "lambda1_connection"
            } else {
                break;
            };
            self.advance();
            let v = match self.tokens.get(self.pos) {
                Some(Token::Number(n)) => {
                    let v = *n;
                    self.pos += 1;
                    v
                }
                other => {
                    return Err(format!(
                        "DEPTH: expected number after {} keyword, got {other:?}",
                        kw.to_uppercase()
                    ))
                }
            };
            let cfg = overrides.get_or_insert_with(crate::curvature::DepthConfig::default);
            match kw {
                "k_metric" => cfg.k_metric = v,
                "k_connection" => cfg.k_connection = v,
                "lambda1_topological" => cfg.lambda1_topological = v,
                "lambda1_connection" => cfg.lambda1_connection = v,
                _ => unreachable!(),
            }
        }
        Ok(Statement::Depth { bundle, config: overrides })
    }

    /// PERCEIVE <bundle>
    ///   ROTATION (r00, r01, ..., r_{N²-1})
    ///   VECTOR   (v0, v1, ..., v_{N-1})
    ///   [DIM N]
    ///
    /// Davis PERCEIVE (Theorem 8.6 — Cognitive Geometry Correspondence).
    /// Returns the perception bias `‖R - I‖_F` as a GQL scalar; the full
    /// (v_perceived, bias) pair is available on the HTTP surface
    /// POST /v1/bundles/{name}/perceive and via the Rust
    /// `curvature::perceive` API.
    ///
    /// `dim` is optional: defaults to `vector.len()`. When supplied,
    /// the executor validates `rotation.len() == dim*dim` and
    /// `vector.len() == dim` and returns PerceiveError variants
    /// translated into GQL execution errors. ROTATION and VECTOR may
    /// appear in either order.
    fn parse_perceive_stmt(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let mut rotation: Option<Vec<f64>> = None;
        let mut vector: Option<Vec<f64>> = None;
        let mut dim: Option<usize> = None;

        loop {
            if self.is_keyword("ROTATION") {
                self.advance();
                self.expect(Token::LParen)?;
                rotation = Some(self.parse_inner_number_list()?);
            } else if self.is_keyword("VECTOR") {
                self.advance();
                self.expect(Token::LParen)?;
                vector = Some(self.parse_inner_number_list()?);
            } else if self.is_keyword("DIM") {
                self.advance();
                let n = match self.tokens.get(self.pos) {
                    Some(Token::Number(n)) => {
                        let v = *n;
                        self.pos += 1;
                        v
                    }
                    other => {
                        return Err(format!("PERCEIVE: expected dim integer after DIM, got {other:?}"));
                    }
                };
                if n < 1.0 || n.fract() != 0.0 {
                    return Err(format!(
                        "PERCEIVE: DIM must be a positive integer, got {}",
                        n
                    ));
                }
                dim = Some(n as usize);
            } else {
                break;
            }
        }

        let rotation = rotation.ok_or_else(|| {
            "PERCEIVE: ROTATION (r00, r01, ...) clause is required".to_string()
        })?;
        let vector = vector.ok_or_else(|| {
            "PERCEIVE: VECTOR (v0, v1, ...) clause is required".to_string()
        })?;
        Ok(Statement::Perceive { bundle, rotation, vector, dim })
    }

    /// Parse `(n0, n1, n2, ...)` after the opening `(` has been consumed.
    /// Returns the inner list of f64 values; the closing `)` is consumed
    /// when encountered. Used by PERCEIVE for ROTATION + VECTOR clauses.
    fn parse_inner_number_list(&mut self) -> Result<Vec<f64>, String> {
        let mut nums = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                self.advance();
                break;
            }
            if !nums.is_empty() {
                self.expect(Token::Comma)?;
            }
            match self.tokens.get(self.pos) {
                Some(Token::Number(n)) => {
                    nums.push(*n);
                    self.pos += 1;
                }
                other => {
                    return Err(format!(
                        "expected number in list, got {other:?}"
                    ));
                }
            }
        }
        Ok(nums)
    }

    fn parse_geodesic(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let from_keys = self.parse_kv_pairs()?;
        if from_keys.is_empty() {
            return Err("GEODESIC: expected key=value pairs after FROM".into());
        }
        self.expect_keyword("TO")?;
        let to_keys = self.parse_kv_pairs()?;
        if to_keys.is_empty() {
            return Err("GEODESIC: expected key=value pairs after TO".into());
        }
        let mut max_hops = 50;
        if self.is_keyword("MAX_HOPS") {
            self.advance();
            max_hops = self.expect_usize()?;
        }
        let mut restrict_bundle = None;
        if self.is_keyword("RESTRICT") {
            self.advance();
            self.expect_keyword("TO")?;
            restrict_bundle = Some(self.expect_word()?);
        }
        Ok(Statement::Geodesic {
            bundle,
            from_keys,
            to_keys,
            max_hops,
            restrict_bundle,
        })
    }

    fn parse_metric_tensor(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        Ok(Statement::MetricTensor { bundle })
    }

    // ── GQL: COMPLETE / PROPAGATE ──

    fn parse_complete(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        let mut where_conditions = Vec::new();
        let mut method = None;
        let mut min_confidence = None;
        let mut with_provenance = false;
        let mut with_constraint_graph = false;

        if self.is_keyword("WHERE") {
            self.advance();
            where_conditions = self.parse_filter_conditions()?;
        }
        if self.is_keyword("METHOD") {
            self.advance();
            method = Some(self.expect_word()?);
        }
        if self.is_keyword("MIN_CONFIDENCE") {
            self.advance();
            match self.advance() {
                Some(Token::Number(n)) => min_confidence = Some(n),
                other => return Err(format!("Expected confidence number, got {other:?}")),
            }
        }
        if self.is_keyword("WITH") {
            self.advance();
            loop {
                let kw = self.expect_word()?;
                match kw.to_ascii_uppercase().as_str() {
                    "PROVENANCE" => with_provenance = true,
                    "CONSTRAINT_GRAPH" => with_constraint_graph = true,
                    _ => return Err(format!("Unknown WITH option: {kw}")),
                }
                if !matches!(self.peek(), Some(Token::Comma)) {
                    break;
                }
                self.advance();
            }
        }

        Ok(Statement::Complete {
            bundle,
            where_conditions,
            method,
            min_confidence,
            with_provenance,
            with_constraint_graph,
        })
    }

    fn parse_propagate(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("ASSUMING")?;
        let assumptions = self.parse_kv_pairs()?;
        // Optional: SHOW NEWLY_DETERMINED (ignored — always returned)
        if self.is_keyword("SHOW") {
            self.advance();
            if self.is_keyword("NEWLY_DETERMINED") {
                self.advance();
            }
        }
        Ok(Statement::Propagate {
            bundle,
            assumptions,
        })
    }

    // ── GQL: SUGGEST_ADJACENCY ──

    fn parse_suggest_adjacency(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;

        let mut fields = Vec::new();
        let mut sample_size = 10_000_usize;
        let mut candidates = 5_usize;

        loop {
            if self.is_keyword("FIELDS") {
                self.advance();
                // Parse comma-separated field list
                loop {
                    fields.push(self.expect_word()?);
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            } else if self.is_keyword("SAMPLE_SIZE") {
                self.advance();
                sample_size = self.expect_usize()?;
            } else if self.is_keyword("CANDIDATES") {
                self.advance();
                candidates = self.expect_usize()?;
            } else if self.is_keyword("MINIMIZING") {
                self.advance();
                self.expect_keyword("h1")?; // only h1 for now
            } else {
                break;
            }
        }

        Ok(Statement::SuggestAdjacency {
            bundle,
            fields,
            sample_size,
            candidates,
        })
    }

    // ── GQL: EXPLAIN ──

    fn parse_explain(&mut self) -> Result<Statement, String> {
        let inner = self.parse()?;
        Ok(Statement::Explain {
            inner: Box::new(inner),
        })
    }

    // ── GQL: EXISTS ──

    fn parse_exists(&mut self) -> Result<Statement, String> {
        self.expect_keyword("SECTION")?;
        let name = self.expect_word()?;
        self.expect_keyword("AT")?;
        let key = self.parse_kv_pairs()?;
        Ok(Statement::ExistsSection { bundle: name, key })
    }

    // ── GQL: ATLAS (transactions) ──

    fn parse_atlas(&mut self) -> Result<Statement, String> {
        let action = self.expect_word()?;
        match action.to_ascii_uppercase().as_str() {
            "BEGIN" => Ok(Statement::AtlasBegin),
            "COMMIT" => Ok(Statement::AtlasCommit),
            "ROLLBACK" => Ok(Statement::AtlasRollback),
            _ => Err(format!("Unknown ATLAS action: {action}")),
        }
    }

    // ── SQL compat: CREATE BUNDLE ──

    fn parse_create_bundle(&mut self) -> Result<Statement, String> {
        self.expect_keyword("BUNDLE")?;
        let name = self.expect_word()?;
        self.parse_bundle_fields_paren(name)
    }

    fn parse_bundle_fields_paren(&mut self, name: String) -> Result<Statement, String> {
        self.expect(Token::LParen)?;

        let mut base_fields = Vec::new();
        let mut fiber_fields = Vec::new();
        let mut indexed = Vec::new();

        loop {
            if self.is_keyword("BASE") || self.is_keyword("FIBER") || self.is_keyword("INDEX") {
                break;
            }
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !base_fields.is_empty() || !fiber_fields.is_empty() {
                self.expect(Token::Comma)?;
            }

            let fname = self.expect_word()?;
            let ftype = self.expect_word()?;
            let mut range = None;

            if self.is_keyword("RANGE") {
                self.advance();
                self.expect(Token::LParen)?;
                match self.advance() {
                    Some(Token::Number(n)) => range = Some(n),
                    other => return Err(format!("Expected range value, got {other:?}")),
                }
                self.expect(Token::RParen)?;
            }

            let mut spec = FieldSpec {
                name: fname,
                ftype: ftype.clone(),
                range,
                default: None,
                auto_inc: false,
                unique: false,
                required: false,
                encryption: crate::types::EncryptionMode::None,
                encryption_group: None,
            };

            if self.is_keyword("BASE") {
                self.advance();
                // v0.2: per-field ENCRYPTED [MODE] may appear AFTER the
                // BASE/FIBER keyword in the SQL-compat syntax. Parse it now
                // so the mode rides on the spec when we push.
                if self.is_keyword("ENCRYPTED") {
                    self.advance();
                    let (mode, group) = self.parse_encryption_mode_and_group(&ftype)?;
                    spec.encryption = mode;
                    spec.encryption_group = group;
                }
                base_fields.push(spec);
            } else if self.is_keyword("FIBER") {
                self.advance();
                if self.is_keyword("ENCRYPTED") {
                    self.advance();
                    let (mode, group) = self.parse_encryption_mode_and_group(&ftype)?;
                    spec.encryption = mode;
                    spec.encryption_group = group;
                }
                fiber_fields.push(spec);
            } else if base_fields.is_empty() {
                if self.is_keyword("ENCRYPTED") {
                    self.advance();
                    let (mode, group) = self.parse_encryption_mode_and_group(&ftype)?;
                    spec.encryption = mode;
                    spec.encryption_group = group;
                }
                base_fields.push(spec);
            } else {
                if self.is_keyword("ENCRYPTED") {
                    self.advance();
                    let (mode, group) = self.parse_encryption_mode_and_group(&ftype)?;
                    spec.encryption = mode;
                    spec.encryption_group = group;
                }
                fiber_fields.push(spec);
            }

            if self.is_keyword("INDEX") {
                self.advance();
                let last = if fiber_fields.is_empty() {
                    base_fields.last().unwrap()
                } else {
                    fiber_fields.last().unwrap()
                };
                indexed.push(last.name.clone());
            }
        }

        self.expect(Token::RParen)?;
        let encrypted = self.is_keyword("ENCRYPTED");
        if encrypted {
            self.advance();
        }

        // ADJACENCY clauses after SQL-style CREATE BUNDLE are also supported
        let mut adjacencies = Vec::new();
        while self.is_keyword("ADJACENCY") {
            self.advance();
            adjacencies.push(self.parse_adjacency_spec()?);
        }

        // INVARIANT field = value +/- tol
        let invariants = self.parse_invariant_specs();

        // v0.2: WITH ENCRYPTION SEED clause may follow.
        let seed_source = self.parse_optional_encryption_seed_clause()?;

        Ok(Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
            invariants,
            seed_source,
        })
    }

    fn parse_invariant_specs(&mut self) -> Vec<InvariantSpec> {
        let mut invariants = Vec::new();
        while self.is_keyword("INVARIANT") {
            self.advance();
            let field = match self.expect_word() {
                Ok(f) => f,
                Err(_) => break,
            };
            if self.expect(Token::Eq).is_err() { break; }
            let expected = match self.advance() {
                Some(Token::Number(n)) => n,
                _ => break,
            };
            // Optional +/- tol: handles `+ / -`, `+-`, `+/-`, or plain number
            let tol = if matches!(self.peek(), Some(Token::Plus)) {
                self.advance(); // consume +
                if matches!(self.peek(), Some(Token::Slash)) {
                    self.advance(); // consume /
                    if matches!(self.peek(), Some(Token::Minus)) {
                        self.advance(); // consume -
                    }
                }
                match self.advance() {
                    Some(Token::Number(n)) => n.abs(),
                    _ => 1e-9,
                }
            } else if matches!(self.peek(), Some(Token::Word(w)) if w == "+-" || w == "+/-") {
                self.advance();
                match self.advance() {
                    Some(Token::Number(n)) => n,
                    _ => 1e-9,
                }
            } else {
                1e-9
            };
            invariants.push(InvariantSpec { field, expected, tol });
        }
        invariants
    }

    fn parse_sql_insert(&mut self) -> Result<Statement, String> {
        self.expect_keyword("INTO")?;
        let bundle = self.expect_word()?;

        let mut columns = Vec::new();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            loop {
                columns.push(self.expect_word()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
        }

        self.expect_keyword("VALUES")?;
        self.expect(Token::LParen)?;

        let mut values = Vec::new();
        loop {
            values.push(self.parse_literal()?);
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            self.expect(Token::Comma)?;
        }
        self.expect(Token::RParen)?;

        Ok(Statement::Insert {
            bundle,
            columns,
            values,
        })
    }

    // ── SQL compat: SELECT ──

    fn parse_sql_select(&mut self) -> Result<Statement, String> {
        let mut columns = Vec::new();
        loop {
            if self.is_keyword("FROM") {
                break;
            }
            if !columns.is_empty() {
                self.expect(Token::Comma)?;
            }
            columns.push(self.parse_select_col()?);
        }

        self.expect_keyword("FROM")?;
        let bundle = self.expect_word()?;

        if self.is_keyword("JOIN") {
            self.advance();
            let right = self.expect_word()?;
            self.expect_keyword("ON")?;
            let on_field = self.expect_word()?;
            return Ok(Statement::Join {
                left: bundle,
                right,
                on_field,
                columns,
            });
        }

        let mut condition = None;
        let mut group_by = None;

        if self.is_keyword("WHERE") {
            self.advance();
            condition = Some(self.parse_sql_condition()?);
        }

        if self.is_keyword("GROUP") {
            self.advance();
            self.expect_keyword("BY")?;
            group_by = Some(self.expect_word()?);
        }

        Ok(Statement::Select {
            bundle,
            columns,
            condition,
            group_by,
        })
    }

    fn parse_select_col(&mut self) -> Result<SelectCol, String> {
        if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            return Ok(SelectCol::Star);
        }

        let word = self.expect_word()?;
        let upper = word.to_ascii_uppercase();

        let agg = match upper.as_str() {
            "COUNT" => Some(AggFunc::Count),
            "SUM" => Some(AggFunc::Sum),
            "AVG" => Some(AggFunc::Avg),
            "MIN" => Some(AggFunc::Min),
            "MAX" => Some(AggFunc::Max),
            _ => None,
        };

        if let Some(func) = agg {
            if matches!(self.peek(), Some(Token::LParen)) {
                self.advance();
                let field = self.expect_word()?;
                self.expect(Token::RParen)?;
                return Ok(SelectCol::Agg(func, field));
            }
        }

        Ok(SelectCol::Name(word))
    }

    fn parse_sql_condition(&mut self) -> Result<Condition, String> {
        let field = self.expect_word()?;

        if self.is_keyword("BETWEEN") {
            self.advance();
            let lo = self.parse_literal()?;
            self.expect_keyword("AND")?;
            let hi = self.parse_literal()?;
            return Ok(Condition::Between(field, lo, hi));
        }

        if self.is_keyword("IN") {
            self.advance();
            self.expect(Token::LParen)?;
            let mut vals = Vec::new();
            loop {
                vals.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
            return Ok(Condition::In(field, vals));
        }

        self.expect(Token::Eq)?;
        let val = self.parse_literal()?;
        Ok(Condition::Eq(field, val))
    }

    // ── Helper: parse key=value pairs ──

    fn parse_kv_pairs(&mut self) -> Result<Vec<(String, Literal)>, String> {
        let mut pairs = Vec::new();
        loop {
            if self.at_end() {
                break;
            }
            // Stop at known clause keywords
            if self.is_keyword("SET")
                || self.is_keyword("PROJECT")
                || self.is_keyword("RANK")
                || self.is_keyword("FIRST")
                || self.is_keyword("SKIP")
                || self.is_keyword("ON")
                || self.is_keyword("WHERE")
                || self.is_keyword("MEASURE")
                || self.is_keyword("OVER")
                || self.is_keyword("UPSERT")
            {
                break;
            }
            if !pairs.is_empty() {
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
            let key = self.expect_word()?;
            // Accept = or :
            if matches!(self.peek(), Some(Token::Eq)) || matches!(self.peek(), Some(Token::Colon)) {
                self.advance();
            } else {
                return Err(format!("Expected '=' or ':' after '{key}'"));
            }
            let val = self.parse_literal()?;
            pairs.push((key, val));
        }
        Ok(pairs)
    }

    fn parse_kv_pairs_inner(&mut self) -> Result<Vec<(String, Literal)>, String> {
        let mut pairs = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !pairs.is_empty() {
                self.expect(Token::Comma)?;
            }
            let key = self.expect_word()?;
            if matches!(self.peek(), Some(Token::Eq)) || matches!(self.peek(), Some(Token::Colon)) {
                self.advance();
            } else {
                return Err(format!("Expected '=' or ':' after '{key}'"));
            }
            let val = self.parse_literal()?;
            pairs.push((key, val));
        }
        Ok(pairs)
    }

    // ── Helper: filter conditions ──

    fn parse_filter_conditions(&mut self) -> Result<Vec<FilterCondition>, String> {
        // Consume ON or WHERE keyword
        if self.is_keyword("ON") || self.is_keyword("WHERE") {
            self.advance();
        }
        self.parse_filter_condition_list()
    }

    fn parse_filter_condition_list(&mut self) -> Result<Vec<FilterCondition>, String> {
        let mut conditions = Vec::new();
        loop {
            // EXISTS (COVER bundle WHERE ...) subquery condition
            if self.is_keyword("EXISTS") {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect_keyword("COVER")?;
                let cover_bundle = self.expect_word()?;
                let where_conds = if self.is_keyword("WHERE") {
                    self.advance();
                    self.parse_filter_condition_list()?
                } else {
                    vec![]
                };
                self.expect(Token::RParen)?;
                conditions.push(FilterCondition::Exists { cover_bundle, where_conds });
            } else {
                conditions.push(self.parse_single_filter()?);
            }
            if self.is_keyword("AND") {
                self.advance();
            } else {
                break;
            }
        }
        Ok(conditions)
    }

    fn parse_single_filter(&mut self) -> Result<FilterCondition, String> {
        let field = self.expect_word()?;

        // Check for VOID / DEFINED
        if field.eq_ignore_ascii_case("VOID") || field.eq_ignore_ascii_case("DEFINED") {
            return Err("VOID/DEFINED must follow a field name".into());
        }

        // field VOID / field DEFINED
        if self.is_keyword("VOID") {
            self.advance();
            return Ok(FilterCondition::Void(field));
        }
        if self.is_keyword("DEFINED") {
            self.advance();
            return Ok(FilterCondition::Defined(field));
        }

        // field MATCHES 'pattern'
        if self.is_keyword("MATCHES") {
            self.advance();
            let pattern = match self.advance() {
                Some(Token::Str(s)) => s,
                other => return Err(format!("Expected string pattern, got {other:?}")),
            };
            return Ok(FilterCondition::Matches(field, pattern));
        }

        // field CONTAINS 'text'
        if self.is_keyword("CONTAINS") {
            self.advance();
            let text = match self.advance() {
                Some(Token::Str(s)) => s,
                other => return Err(format!("Expected string, got {other:?}")),
            };
            return Ok(FilterCondition::Contains(field, text));
        }

        // field BETWEEN lo AND hi
        if self.is_keyword("BETWEEN") {
            self.advance();
            let lo = self.parse_literal()?;
            self.expect_keyword("AND")?;
            let hi = self.parse_literal()?;
            return Ok(FilterCondition::Between(field, lo, hi));
        }

        // field IN (v1, v2, ...)
        if self.is_keyword("IN") {
            self.advance();
            self.expect(Token::LParen)?;
            let mut vals = Vec::new();
            loop {
                vals.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
            return Ok(FilterCondition::In(field, vals));
        }

        // field NOT IN (v1, v2, ...)
        if self.is_keyword("NOT") {
            self.advance();
            self.expect_keyword("IN")?;
            self.expect(Token::LParen)?;
            let mut vals = Vec::new();
            loop {
                vals.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
            return Ok(FilterCondition::NotIn(field, vals));
        }

        // Comparison operators
        match self.peek() {
            Some(Token::Eq) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Eq(field, val))
            }
            Some(Token::Neq) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Neq(field, val))
            }
            Some(Token::Gt) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Gt(field, val))
            }
            Some(Token::Gte) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Gte(field, val))
            }
            Some(Token::Lt) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Lt(field, val))
            }
            Some(Token::Lte) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Lte(field, val))
            }
            other => Err(format!(
                "Expected comparison operator after '{field}', got {other:?}"
            )),
        }
    }

    // ── Helper: sort specs ──

    fn parse_sort_specs(&mut self) -> Result<Vec<SortSpec>, String> {
        let mut specs = Vec::new();
        loop {
            let field = self.expect_word()?;
            let desc = if self.is_keyword("DESC") {
                self.advance();
                true
            } else {
                if self.is_keyword("ASC") {
                    self.advance();
                }
                false
            };
            specs.push(SortSpec { field, desc });
            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }
            self.advance();
        }
        Ok(specs)
    }

    // ── Helper: name list ──

    fn parse_name_list(&mut self) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !names.is_empty() {
                    self.expect(Token::Comma)?;
                }
                names.push(self.expect_word()?);
            }
            self.expect(Token::RParen)?;
        } else {
            names.push(self.expect_word()?);
        }
        Ok(names)
    }

    fn parse_usize(&mut self) -> Result<usize, String> {
        match self.advance() {
            Some(Token::Number(n)) if n >= 0.0 => Ok(n as usize),
            other => Err(format!("Expected positive integer, got {other:?}")),
        }
    }

    // ── v2.1: SHOW (extended) ──

    fn parse_show(&mut self) -> Result<Statement, String> {
        let what = self.expect_word()?;
        match what.to_ascii_uppercase().as_str() {
            "BUNDLES" => {
                let _verbose = self.is_keyword("VERBOSE");
                if _verbose {
                    self.advance();
                }
                Ok(Statement::ShowBundles)
            }
            "ROLES" => Ok(Statement::ShowRoles),
            // Ask G — `SHOW PATTERNS` returns the in-process pattern registry.
            #[cfg(feature = "patterns")]
            "PATTERNS" => Ok(Statement::ShowPatterns),
            "PREPARED" => Ok(Statement::ShowPrepared),
            "BACKUPS" => Ok(Statement::ShowBackups),
            "SETTINGS" => Ok(Statement::ShowSettings),
            "SESSION" => Ok(Statement::ShowSession),
            "CURRENT" => {
                self.expect_keyword("ROLE")?;
                Ok(Statement::ShowCurrentRole)
            }
            "FIELDS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowFields { bundle })
            }
            "INDEXES" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowIndexes { bundle })
            }
            "CONSTRAINTS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowConstraints { bundle })
            }
            "MORPHISMS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowMorphisms { bundle })
            }
            "TRIGGERS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowTriggers { bundle })
            }
            "POLICIES" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowPolicies { bundle })
            }
            "STATISTICS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowStatistics { bundle })
            }
            "GEOMETRY" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowGeometry { bundle })
            }
            "COMMENTS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowComments { bundle })
            }
            // `SHOW LATTICE name` — gated on the `lattice` feature
            // (general primitive; first consumer is Halcyon).
            #[cfg(feature = "lattice")]
            "LATTICE" => {
                let name = self.expect_word()?;
                Ok(Statement::ShowLattice { name })
            }
            // `SHOW GAUGE_FIELD name [BUFFER]` — gated on `gauge`.
            // Closes the SHOW half of TDD-HAL-II.5. Without `BUFFER`
            // the row carries only metadata; with `BUFFER` the row
            // additionally carries the full (n_edges, repr_dim)
            // matrix in the `data` column.
            #[cfg(feature = "gauge")]
            "GAUGE_FIELD" => {
                let name = self.expect_word()?;
                let with_buffer = self.is_keyword("BUFFER");
                if with_buffer {
                    self.advance();
                }
                Ok(Statement::ShowGaugeField { name, with_buffer })
            }
            // `SHOW E_FIELD name [BUFFER]` — gated on `gauge`. Closes
            // the SHOW half of TDD-HAL-IV.7. Without `BUFFER` the row
            // carries only metadata (name/source_gauge_field/
            // source_lattice/group/n_edges/init_kind/init_seed); with
            // `BUFFER` the row additionally carries the full
            // `(n_edges, 4)` Lie buffer in the `data` column.
            #[cfg(feature = "gauge")]
            "E_FIELD" => {
                let name = self.expect_word()?;
                let with_buffer = self.is_keyword("BUFFER");
                if with_buffer {
                    self.advance();
                }
                Ok(Statement::ShowEField { name, with_buffer })
            }
            _ => Err(format!("Unknown SHOW target: {what}")),
        }
    }

    // ── v2.1: Access Control ──

    fn parse_weave(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ROLE")?;
        let name = self.expect_word()?;
        let mut password = None;
        let mut inherits = None;
        let mut superweave = false;
        while !self.at_end() {
            if self.is_keyword("PASSWORD") {
                self.advance();
                match self.advance() {
                    Some(Token::Str(s)) => password = Some(s),
                    _ => return Err("Expected password string".into()),
                }
            } else if self.is_keyword("INHERITS") {
                self.advance();
                inherits = Some(self.expect_word()?);
            } else if self.is_keyword("SUPERWEAVE") {
                self.advance();
                superweave = true;
            } else {
                break;
            }
        }
        Ok(Statement::WeaveRole {
            name,
            password,
            inherits,
            superweave,
        })
    }

    fn parse_unweave(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ROLE")?;
        let name = self.expect_word()?;
        Ok(Statement::UnweaveRole { name })
    }

    fn parse_grant(&mut self) -> Result<Statement, String> {
        let mut operations = vec![self.expect_word()?];
        while self.is_keyword(",") || matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            operations.push(self.expect_word()?);
        }
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("TO")?;
        let role = self.expect_word()?;
        Ok(Statement::Grant {
            operations,
            bundle,
            role,
        })
    }

    fn parse_revoke(&mut self) -> Result<Statement, String> {
        let mut operations = vec![self.expect_word()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            operations.push(self.expect_word()?);
        }
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let role = self.expect_word()?;
        Ok(Statement::Revoke {
            operations,
            bundle,
            role,
        })
    }

    fn parse_policy(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("FOR")?;
        let mut operations = vec![self.expect_word()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            operations.push(self.expect_word()?);
        }
        self.expect_keyword("RESTRICT")?;
        self.expect_keyword("TO")?;
        // Capture the rest as the restrict query string
        let mut restrict_parts = Vec::new();
        let mut depth = 0i32;
        while !self.at_end() {
            if self.is_keyword("TO") && depth == 0 {
                break;
            }
            match self.peek() {
                Some(Token::LParen) => {
                    depth += 1;
                    restrict_parts.push("(".to_string());
                    self.advance();
                }
                Some(Token::RParen) => {
                    depth -= 1;
                    restrict_parts.push(")".to_string());
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                }
                Some(Token::Word(w)) => {
                    restrict_parts.push(w.clone());
                    self.advance();
                }
                Some(Token::Str(s)) => {
                    restrict_parts.push(format!("'{s}'"));
                    self.advance();
                }
                Some(Token::Number(n)) => {
                    restrict_parts.push(n.to_string());
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
        let restrict_query = restrict_parts.join(" ");
        self.expect_keyword("TO")?;
        let role = self.expect_word()?;
        Ok(Statement::CreatePolicy {
            name,
            bundle,
            operations,
            restrict_query,
            role,
        })
    }

    fn parse_drop(&mut self) -> Result<Statement, String> {
        let what = self.expect_word()?;
        match what.to_ascii_uppercase().as_str() {
            "BUNDLE" => {
                let name = self.expect_word()?;
                Ok(Statement::Collapse { bundle: name })
            }
            "POLICY" => {
                let name = self.expect_word()?;
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::DropPolicy { name, bundle })
            }
            "TRIGGER" => {
                let name = self.expect_word()?;
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::DropTrigger { name, bundle })
            }
            // Ask G — DROP PATTERN <name>.
            #[cfg(feature = "patterns")]
            "PATTERN" => {
                let name = self.expect_word()?;
                Ok(Statement::DropPattern { name })
            }
            _ => Err(format!("Unknown DROP target: {what}")),
        }
    }

    // ─── Ask G: PATTERN_HUNT parsing (Phase 1) ────────────────────────────
    //
    // Per `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §3. All four methods below
    // are gated on the `patterns` Cargo feature; the default build does
    // not see them and the dispatcher arms for DEFINE/HUNT and the
    // SHOW PATTERNS / DROP PATTERN extensions are themselves gated.
    //
    // **Domain-neutral by construction.** Nothing in here references a
    // single consumer's vocabulary. Field names, pattern names, and bundle
    // names are all opaque identifiers that the parser stores verbatim.
    // The grammar serves vuln-hunt (SCJ), fraud-detection (PRISM), at-risk
    // identification, discourse-flow (Marcella), or any future consumer
    // that wants weighted predicate-filtered ranked queries.

    /// `DEFINE [OR REPLACE] PATTERN <name> AS <pred> [WEIGHT (<expr>)] [USING (<field>,...)]`.
    /// Already consumed the leading `DEFINE` keyword.
    ///
    /// `OR REPLACE` (between DEFINE and PATTERN) opts the statement into
    /// silent overwrite of any pre-existing pattern with the same name.
    /// Without it, the collision is a typed error at execute time (PH6a).
    #[cfg(feature = "patterns")]
    fn parse_define_pattern(&mut self) -> Result<Statement, String> {
        // Optional OR REPLACE.
        let replace = if self.is_keyword("OR") {
            self.advance();
            self.expect_keyword("REPLACE")?;
            true
        } else {
            false
        };

        self.expect_keyword("PATTERN")?;
        let name = self.expect_word()?;
        self.expect_keyword("AS")?;
        let pred = self.parse_filter_condition_list()?;

        let mut weight: Option<Vec<String>> = None;
        let mut using_fields: Vec<String> = Vec::new();

        // WEIGHT and USING are optional and can appear in either order.
        // Loop until we see neither.
        loop {
            if self.is_keyword("WEIGHT") {
                self.advance();
                weight = Some(self.collect_paren_body_tokens()?);
            } else if self.is_keyword("USING") {
                self.advance();
                using_fields = self.parse_field_list_in_parens()?;
            } else {
                break;
            }
        }

        Ok(Statement::DefinePattern {
            name,
            pred,
            // OR groups land in a follow-up sub-spec; v0.1 grammar uses
            // explicit OR inside the predicate body, parsed by
            // parse_filter_condition_list above.
            or_groups: Vec::new(),
            weight,
            using_fields,
            replace,
        })
    }

    /// `HUNT <pattern> IN <bundle> [EXCLUDING IN <b>]* [TOP <n>] [PROJECT (...)]`.
    /// Already consumed the leading `HUNT` keyword.
    #[cfg(feature = "patterns")]
    fn parse_hunt(&mut self) -> Result<Statement, String> {
        let pattern = self.expect_word()?;
        self.expect_keyword("IN")?;
        let bundle = self.expect_word()?;

        let mut excluding: Vec<String> = Vec::new();
        let mut top: Option<usize> = None;
        let mut project: Option<Vec<String>> = None;

        // Optional clauses, in any order. Phase 1 handles EXCLUDING IN,
        // TOP, and PROJECT. extra_on / extra_where / rank_by are part of
        // the spec EBNF but default to empty in Phase 1 — wiring them
        // through is a follow-up gate that doesn't change the AST shape.
        loop {
            if self.is_keyword("EXCLUDING") {
                self.advance();
                self.expect_keyword("IN")?;
                excluding.push(self.expect_word()?);
            } else if self.is_keyword("TOP") {
                self.advance();
                match self.advance() {
                    Some(Token::Number(n)) => top = Some(n as usize),
                    other => {
                        return Err(format!(
                            "Expected number after TOP, got {other:?}"
                        ));
                    }
                }
            } else if self.is_keyword("PROJECT") {
                self.advance();
                project = Some(self.parse_field_list_in_parens()?);
            } else {
                break;
            }
        }

        Ok(Statement::Hunt {
            pattern,
            bundle,
            excluding,
            extra_on: Vec::new(),
            extra_where: Vec::new(),
            rank_by: None,
            top,
            project,
        })
    }

    /// Collect raw tokens between matched parentheses, returning their
    /// canonical string forms. The opening `(` is consumed by this
    /// function; the matching closing `)` is consumed and NOT included.
    ///
    /// Used by the `WEIGHT (...)` clause in DEFINE PATTERN — Phase 1
    /// stores WEIGHT as a token list; Phase 3's evaluator does the
    /// arithmetic-AST parse. This split lets the parser ship without
    /// committing to the WeightExpr enum shape (spec §9.3, OQ-2).
    #[cfg(feature = "patterns")]
    fn collect_paren_body_tokens(&mut self) -> Result<Vec<String>, String> {
        self.expect(Token::LParen)?;
        let mut tokens: Vec<String> = Vec::new();
        let mut depth: usize = 1;
        loop {
            let tok = self
                .advance()
                .ok_or_else(|| "Unexpected end of input inside (...) body".to_string())?;
            match &tok {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            let rendered = match tok {
                Token::Word(w) => w,
                Token::Number(n) => n.to_string(),
                Token::Str(s) => format!("'{s}'"),
                Token::LParen => "(".to_string(),
                Token::RParen => ")".to_string(),
                Token::LBrace => "{".to_string(),
                Token::RBrace => "}".to_string(),
                Token::Comma => ",".to_string(),
                Token::Eq => "=".to_string(),
                Token::Neq => "!=".to_string(),
                Token::Gt => ">".to_string(),
                Token::Gte => ">=".to_string(),
                Token::Lt => "<".to_string(),
                Token::Lte => "<=".to_string(),
                Token::Star => "*".to_string(),
                Token::Slash => "/".to_string(),
                Token::Dot => ".".to_string(),
                Token::Colon => ":".to_string(),
                Token::Semicolon => ";".to_string(),
                Token::Plus => "+".to_string(),
                Token::Minus => "-".to_string(),
            };
            tokens.push(rendered);
        }
        Ok(tokens)
    }

    /// Parse `(field1, field2, field3)` — a parenthesized, comma-separated
    /// list of identifier names. Used by `USING (...)` in DEFINE PATTERN
    /// and `PROJECT (...)` in HUNT. The empty list `()` is accepted.
    #[cfg(feature = "patterns")]
    fn parse_field_list_in_parens(&mut self) -> Result<Vec<String>, String> {
        self.expect(Token::LParen)?;
        let mut fields: Vec<String> = Vec::new();
        // Empty list shortcut.
        if matches!(self.peek(), Some(Token::RParen)) {
            self.advance();
            return Ok(fields);
        }
        loop {
            fields.push(self.expect_word()?);
            match self.peek() {
                Some(Token::Comma) => {
                    self.advance();
                }
                Some(Token::RParen) => {
                    self.advance();
                    break;
                }
                other => {
                    return Err(format!(
                        "Expected ',' or ')' in field list, got {other:?}"
                    ));
                }
            }
        }
        Ok(fields)
    }

    fn parse_audit(&mut self) -> Result<Statement, String> {
        // AUDIT SHOW bundle ... or AUDIT bundle ON/OFF
        let next = self.expect_word()?;
        if next.eq_ignore_ascii_case("SHOW") {
            let bundle = self.expect_word()?;
            let mut since = None;
            let mut role = None;
            while !self.at_end() {
                if self.is_keyword("SINCE") {
                    self.advance();
                    match self.advance() {
                        Some(Token::Str(s)) => since = Some(s),
                        Some(Token::Number(n)) => since = Some(n.to_string()),
                        _ => return Err("Expected date after SINCE".into()),
                    }
                } else if self.is_keyword("ROLE") {
                    self.advance();
                    role = Some(self.expect_word()?);
                } else {
                    break;
                }
            }
            Ok(Statement::AuditShow {
                bundle,
                since,
                role,
            })
        } else {
            let bundle = next;
            let mode = self.expect_word()?;
            if mode.eq_ignore_ascii_case("OFF") {
                Ok(Statement::AuditOff { bundle })
            } else {
                // ON with optional operations list
                let mut operations = Vec::new();
                while !self.at_end() {
                    if self.is_keyword("SECTION")
                        || self.is_keyword("REDEFINE")
                        || self.is_keyword("RETRACT")
                    {
                        if let Some(Token::Word(w)) = self.advance() {
                            operations.push(w.to_ascii_uppercase());
                        }
                    } else {
                        break;
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                Ok(Statement::AuditOn { bundle, operations })
            }
        }
    }

    // ── v2.1: Constraints ──

    fn parse_gauge(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let action = self.expect_word()?;
        match action.to_ascii_uppercase().as_str() {
            "CONSTRAIN" => {
                // Capture everything in parens as constraint text
                self.expect(Token::LParen)?;
                let mut constraints = Vec::new();
                let mut current = String::new();
                let mut depth = 1i32;
                loop {
                    match self.advance() {
                        Some(Token::LParen) => {
                            depth += 1;
                            current.push('(');
                        }
                        Some(Token::RParen) => {
                            depth -= 1;
                            if depth == 0 {
                                if !current.trim().is_empty() {
                                    constraints.push(current.trim().to_string());
                                }
                                break;
                            }
                            current.push(')');
                        }
                        Some(Token::Comma) if depth == 1 => {
                            if !current.trim().is_empty() {
                                constraints.push(current.trim().to_string());
                            }
                            current = String::new();
                        }
                        Some(Token::Word(w)) => {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push_str(&w);
                        }
                        Some(Token::Number(n)) => {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push_str(&n.to_string());
                        }
                        Some(Token::Str(s)) => {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push('\'');
                            current.push_str(&s);
                            current.push('\'');
                        }
                        Some(Token::Eq) => current.push('='),
                        Some(Token::Gt) => current.push('>'),
                        Some(Token::Lt) => current.push('<'),
                        Some(Token::Gte) => current.push_str(">="),
                        Some(Token::Lte) => current.push_str("<="),
                        Some(Token::Neq) => current.push_str("!="),
                        Some(Token::Star) => current.push('*'),
                        Some(Token::Plus) => current.push('+'),
                        Some(Token::Minus) => current.push('-'),
                        None => return Err("Unexpected end in GAUGE CONSTRAIN".into()),
                        _ => {}
                    }
                }
                Ok(Statement::GaugeConstrain {
                    bundle,
                    constraints,
                })
            }
            "UNCONSTRAIN" => {
                let constraint_name = self.expect_word()?;
                Ok(Statement::GaugeUnconstrain {
                    bundle,
                    constraint_name,
                })
            }
            "VS" => {
                // GAUGE bundle1 VS bundle2 ON FIBER (f1, f2) AROUND field
                let bundle2 = bundle;
                // In this syntax: GAUGE was parsed, bundle is bundle1, action was "VS", next is bundle2
                // Actually: GAUGE bundle1 VS bundle2 — `bundle` = bundle1, action = "VS", we need bundle2 next
                let bundle1_name = bundle2; // rename for clarity
                let bundle2_name = self.expect_word()?;
                self.expect_word()?; // ON
                self.expect_word()?; // FIBER
                self.expect(Token::LParen)?;
                let fiber_fields = self.parse_word_list()?;
                self.expect_word()?; // AROUND
                let around_field = self.expect_word()?;
                Ok(Statement::GaugeTest {
                    bundle1: bundle1_name,
                    bundle2: bundle2_name,
                    fiber_fields,
                    around_field,
                })
            }
            "ROTATE_KEY" => {
                // GAUGE <bundle> ROTATE_KEY FORWARD_SECRET [WITH ENCRYPTION SEED ...]
                let mode = self.expect_word()?;
                if mode.to_ascii_uppercase() != "FORWARD_SECRET" {
                    return Err(format!(
                        "Expected FORWARD_SECRET after ROTATE_KEY, got {mode}"
                    ));
                }
                let new_seed_source = self.parse_optional_encryption_seed_clause()?;
                Ok(Statement::RotateKey {
                    bundle,
                    new_seed_source,
                })
            }
            _ => Err(format!(
                "Expected CONSTRAIN, UNCONSTRAIN, VS, or ROTATE_KEY, got {action}"
            )),
        }
    }

    // ── Parallel Transport / Holonomy ──

    fn parse_transport(&mut self) -> Result<Statement, String> {
        // TRANSPORT bundle FROM (k=v, ...) TO (k=v, ...) ON FIBER (f1, f2, ...)
        let bundle = self.expect_word()?;
        self.expect_word()?; // FROM
        self.expect(Token::LParen)?;
        let from_keys = self.parse_kv_pairs_inner()?;
        self.expect(Token::RParen)?;
        self.expect_word()?; // TO
        self.expect(Token::LParen)?;
        let to_keys = self.parse_kv_pairs_inner()?;
        self.expect(Token::RParen)?;
        self.expect_word()?; // ON
        self.expect_word()?; // FIBER
        self.expect(Token::LParen)?;
        let fiber_fields = self.parse_inner_word_list()?;
        Ok(Statement::Transport { bundle, from_keys, to_keys, fiber_fields })
    }

    /// C2 — TRANSPORT_ROTATION bundle
    ///         FROM (k=v, ...) TO (k=v, ...)
    ///         ON FIBER (f0, f1, ..., fN-1)
    ///         WITH ANGLE 0.6
    /// Returns the N×N Rodrigues rotation matrix in the plane spanned
    /// by the FROM/TO fiber vectors, rotated by the supplied angle.
    fn parse_transport_rotation(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_word()?; // FROM
        self.expect(Token::LParen)?;
        let from_keys = self.parse_kv_pairs_inner()?;
        self.expect(Token::RParen)?;
        self.expect_word()?; // TO
        self.expect(Token::LParen)?;
        let to_keys = self.parse_kv_pairs_inner()?;
        self.expect(Token::RParen)?;
        self.expect_word()?; // ON
        self.expect_word()?; // FIBER
        self.expect(Token::LParen)?;
        let fiber_fields = self.parse_inner_word_list()?;
        self.expect_word()?; // WITH
        self.expect_word()?; // ANGLE
        let angle = match self.advance() {
            Some(Token::Number(n)) => n,
            other => return Err(format!(
                "Expected angle number after WITH ANGLE, got {other:?}"
            )),
        };
        Ok(Statement::TransportRotation {
            bundle, from_keys, to_keys, fiber_fields, angle,
        })
    }

    /// S4 — SAMPLE_TRANSPORT verb.
    ///
    /// Grammar:
    /// ```text
    /// SAMPLE_TRANSPORT bundle
    ///   FROM (k=v, ...)
    ///   ON FIBER (f1, f2, ...)
    ///   BUDGET <number>
    ///   N <integer>
    ///   [BETA <number>]
    ///   [SEED <integer>]
    ///   ;
    /// ```
    fn parse_sample_transport(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        self.expect(Token::LParen)?;
        let from_keys = self.parse_kv_pairs_inner()?;
        self.expect(Token::RParen)?;
        self.expect_keyword("ON")?;
        self.expect_keyword("FIBER")?;
        self.expect(Token::LParen)?;
        let fiber_fields = self.parse_inner_word_list()?;
        self.expect(Token::RParen)?;
        self.expect_keyword("BUDGET")?;
        let budget = match self.advance() {
            Some(Token::Number(n)) => n,
            other => return Err(format!("Expected budget number after BUDGET, got {other:?}")),
        };
        self.expect_keyword("N")?;
        let k = self.expect_usize()?;
        let mut beta: Option<f64> = None;
        let mut seed: Option<u64> = None;
        while !self.at_end() {
            if self.is_keyword("BETA") {
                self.advance();
                beta = Some(match self.advance() {
                    Some(Token::Number(n)) => n,
                    other => {
                        return Err(format!("Expected beta number after BETA, got {other:?}"))
                    }
                });
            } else if self.is_keyword("SEED") {
                self.advance();
                seed = Some(match self.advance() {
                    Some(Token::Number(n)) if n >= 0.0 && n.fract() == 0.0 => n as u64,
                    other => {
                        return Err(format!("Expected seed integer after SEED, got {other:?}"))
                    }
                });
            } else {
                break;
            }
        }
        Ok(Statement::SampleTransport {
            bundle,
            from_keys,
            fiber_fields,
            budget,
            k,
            beta,
            seed,
        })
    }

    fn parse_holonomy(&mut self) -> Result<Statement, String> {
        // HOLONOMY bundle ON FIBER (f1, f2) AROUND field
        // HOLONOMY bundle NEAR (f1=v1, ...) WITHIN r [METRIC m] ON FIBER (f1, f2) AROUND field
        let bundle = self.expect_word()?;
        let keyword = self.expect_word()?;
        match keyword.to_ascii_uppercase().as_str() {
            "ON" => {
                self.expect_word()?; // FIBER
                self.expect(Token::LParen)?;
                let fiber_fields = self.parse_inner_word_list()?;
                self.expect_word()?; // AROUND
                let around_field = self.expect_word()?;
                Ok(Statement::HolonomyFiber { bundle, fiber_fields, around_field })
            }
            "NEAR" => {
                self.expect(Token::LParen)?;
                let near_kv = self.parse_kv_pairs_inner()?;
                self.expect(Token::RParen)?;
                // Convert Literal values to f64
                let near_point: Vec<(String, f64)> = near_kv
                    .into_iter()
                    .map(|(k, v)| {
                        let f = match v {
                            Literal::Float(f) => f,
                            Literal::Integer(i) => i as f64,
                            _ => 0.0,
                        };
                        (k, f)
                    })
                    .collect();
                self.expect_word()?; // WITHIN
                let near_radius = match self.advance() {
                    Some(Token::Number(n)) => n,
                    other => return Err(format!("Expected radius number, got {other:?}")),
                };
                // Optional METRIC keyword
                let near_metric = if self.is_keyword("METRIC") {
                    self.advance(); // consume METRIC
                    Some(self.expect_word()?)
                } else {
                    None
                };
                self.expect_word()?; // ON
                self.expect_word()?; // FIBER
                self.expect(Token::LParen)?;
                let fiber_fields = self.parse_inner_word_list()?;
                self.expect_word()?; // AROUND
                let around_field = self.expect_word()?;
                Ok(Statement::LocalHolonomy { bundle, near_point, near_radius, near_metric, fiber_fields, around_field })
            }
            other => Err(format!("Expected ON or NEAR after HOLONOMY bundle, got {other}")),
        }
    }

    /// Parse a comma-separated word list that was already opened with `(`. Consumes the closing `)`.
    fn parse_inner_word_list(&mut self) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                self.advance();
                break;
            }
            if !names.is_empty() {
                self.expect(Token::Comma)?;
            }
            names.push(self.expect_word()?);
        }
        Ok(names)
    }

    /// Alias for parse_inner_word_list used in other contexts.
    fn parse_word_list(&mut self) -> Result<Vec<String>, String> {
        self.parse_inner_word_list()
    }

    // ── v2.1: Maintenance ──

    fn parse_compact(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let analyze = self.is_keyword("ANALYZE");
        if analyze {
            self.advance();
        }
        Ok(Statement::Compact { bundle, analyze })
    }

    fn parse_analyze(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let mut field = None;
        let mut full = false;
        if self.is_keyword("ON") {
            self.advance();
            field = Some(self.expect_word()?);
        } else if self.is_keyword("FULL") {
            self.advance();
            full = true;
        }
        Ok(Statement::Analyze {
            bundle,
            field,
            full,
        })
    }

    fn parse_vacuum(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let full = self.is_keyword("FULL");
        if full {
            self.advance();
        }
        Ok(Statement::Vacuum { bundle, full })
    }

    fn parse_rebuild(&mut self) -> Result<Statement, String> {
        self.expect_keyword("INDEX")?;
        let bundle = self.expect_word()?;
        let mut field = None;
        if self.is_keyword("ON") {
            self.advance();
            field = Some(self.expect_word()?);
        }
        Ok(Statement::RebuildIndex { bundle, field })
    }

    fn parse_check(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        Ok(Statement::CheckIntegrity { bundle })
    }

    // ── v2.1: Session ──

    fn parse_set(&mut self) -> Result<Statement, String> {
        let key = self.expect_word()?;
        let value = self.parse_literal()?;
        Ok(Statement::Set { key, value })
    }

    fn parse_reset(&mut self) -> Result<Statement, String> {
        if self.is_keyword("ALL") {
            self.advance();
            Ok(Statement::Reset { key: None })
        } else {
            let key = self.expect_word()?;
            Ok(Statement::Reset { key: Some(key) })
        }
    }

    // ── v2.1: Data Movement ──

    fn parse_ingest(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let source = match self.advance() {
            Some(Token::Str(s)) => s,
            Some(Token::Word(w)) => w, // STDIN
            other => return Err(format!("Expected source path, got {other:?}")),
        };
        self.expect_keyword("FORMAT")?;
        let format = self.expect_word()?;
        Ok(Statement::Ingest {
            bundle,
            source,
            format,
        })
    }

    fn parse_transplant(&mut self) -> Result<Statement, String> {
        let source = self.expect_word()?;
        self.expect_keyword("INTO")?;
        let target = self.expect_word()?;
        let mut conditions = Vec::new();
        let mut retract_source = false;
        if self.is_keyword("WHERE") {
            self.advance();
            conditions = self.parse_filter_condition_list()?;
        }
        if self.is_keyword("RETRACT") {
            self.advance();
            self.expect_keyword("SOURCE")?;
            retract_source = true;
        }
        Ok(Statement::Transplant {
            source,
            target,
            conditions,
            retract_source,
        })
    }

    fn parse_generate(&mut self) -> Result<Statement, String> {
        self.expect_keyword("BASE")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let field = self.expect_word()?;
        self.expect(Token::Eq)?;
        let from_val = self.parse_literal()?;
        self.expect_keyword("TO")?;
        // skip "field=" again
        let _field2 = self.expect_word()?;
        self.expect(Token::Eq)?;
        let to_val = self.parse_literal()?;
        self.expect_keyword("STEP")?;
        let step = self.parse_literal()?;
        Ok(Statement::GenerateBase {
            bundle,
            field,
            from_val,
            to_val,
            step,
        })
    }

    fn parse_fill(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("ON")?;
        let field = self.expect_word()?;
        self.expect_keyword("USING")?;
        let method = self.expect_word()?;
        // Optionally consume a qualifier like LINEAR
        let method = if self.is_keyword("LINEAR") || self.is_keyword("TRANSPORT") {
            let extra = self.expect_word()?;
            format!("{method} {extra}")
        } else {
            method
        };
        Ok(Statement::Fill {
            bundle,
            field,
            method,
        })
    }

    // ── v2.1: Prepared Statements ──

    fn parse_prepare(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("AS")?;
        // Capture the rest of the tokens as the body string
        let mut parts = Vec::new();
        while !self.at_end() {
            match self.advance() {
                Some(Token::Word(w)) => parts.push(w),
                Some(Token::Number(n)) => parts.push(n.to_string()),
                Some(Token::Str(s)) => parts.push(format!("'{s}'")),
                Some(Token::LParen) => parts.push("(".into()),
                Some(Token::RParen) => parts.push(")".into()),
                Some(Token::Comma) => parts.push(",".into()),
                Some(Token::Eq) => parts.push("=".into()),
                Some(Token::Gt) => parts.push(">".into()),
                Some(Token::Lt) => parts.push("<".into()),
                Some(Token::Gte) => parts.push(">=".into()),
                Some(Token::Lte) => parts.push("<=".into()),
                Some(Token::Neq) => parts.push("!=".into()),
                Some(Token::Star) => parts.push("*".into()),
                Some(Token::Colon) => parts.push(":".into()),
                Some(Token::Plus) => parts.push("+".into()),
                Some(Token::Minus) => parts.push("-".into()),
                Some(Token::Dot) => parts.push(".".into()),
                _ => break,
            }
        }
        let body = parts.join(" ");
        Ok(Statement::Prepare { name, body })
    }

    fn parse_execute(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let mut params = Vec::new();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    self.advance();
                    break;
                }
                params.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
            }
        }
        Ok(Statement::Execute { name, params })
    }

    fn parse_deallocate(&mut self) -> Result<Statement, String> {
        if self.is_keyword("ALL") {
            self.advance();
            Ok(Statement::Deallocate { name: None })
        } else {
            let name = self.expect_word()?;
            Ok(Statement::Deallocate { name: Some(name) })
        }
    }

    // ── v2.1: Backup / Restore ──

    fn parse_backup(&mut self) -> Result<Statement, String> {
        let first = self.expect_word()?;
        let (bundle, all) = if first.eq_ignore_ascii_case("ALL") {
            (None, true)
        } else {
            (Some(first), false)
        };
        self.expect_keyword("TO")?;
        let path = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected path string, got {other:?}")),
        };
        let mut compress = false;
        let mut incremental_since = None;
        while !self.at_end() {
            if self.is_keyword("COMPRESS") {
                self.advance();
                compress = true;
            } else if self.is_keyword("INCREMENTAL") {
                self.advance();
                self.expect_keyword("SINCE")?;
                match self.advance() {
                    Some(Token::Str(s)) => incremental_since = Some(s),
                    _ => return Err("Expected date string after SINCE".into()),
                }
            } else {
                break;
            }
        }
        let bundle_name = if all { None } else { bundle };
        Ok(Statement::Backup {
            bundle: bundle_name,
            path,
            compress,
            incremental_since,
        })
    }

    fn parse_restore(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let path = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected path string, got {other:?}")),
        };
        let mut snapshot = None;
        let mut rename = None;
        while !self.at_end() {
            if self.is_keyword("AT") {
                self.advance();
                self.expect_keyword("SNAPSHOT")?;
                match self.advance() {
                    Some(Token::Str(s)) => snapshot = Some(s),
                    _ => return Err("Expected snapshot name".into()),
                }
            } else if self.is_keyword("AS") {
                self.advance();
                rename = Some(self.expect_word()?);
            } else {
                break;
            }
        }
        Ok(Statement::Restore {
            bundle,
            path,
            snapshot,
            rename,
        })
    }

    fn parse_verify(&mut self) -> Result<Statement, String> {
        self.expect_keyword("BACKUP")?;
        let path = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected path string, got {other:?}")),
        };
        Ok(Statement::VerifyBackup { path })
    }

    // ── v2.1: Comments ──

    fn parse_comment(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let target_type = self.expect_word()?; // BUNDLE, FIELD, CONSTRAINT
        let target = self.expect_word()?;
        // Handle dotted names like sensors.temp
        let target = if matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            let field = self.expect_word()?;
            format!("{target}.{field}")
        } else {
            target
        };
        self.expect_keyword("IS")?;
        let comment = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected comment string, got {other:?}")),
        };
        Ok(Statement::CommentOn {
            target_type,
            target,
            comment,
        })
    }

    // ── v2.1: Recursive ──

    fn parse_iterate(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("START")?;
        self.expect_keyword("AT")?;
        let mut start_key = Vec::new();
        loop {
            let field = self.expect_word()?;
            self.expect(Token::Eq)?;
            let val = self.parse_literal()?;
            start_key.push((field, val));
            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }
            self.advance();
        }
        self.expect_keyword("STEP")?;
        self.expect_keyword("ALONG")?;
        let step_field = self.expect_word()?;
        let mut max_depth = None;
        // consume UNTIL VOID or UNTIL DEPTH n or MAX DEPTH n
        while !self.at_end() {
            if self.is_keyword("UNTIL") {
                self.advance();
                if self.is_keyword("VOID") {
                    self.advance();
                } else if self.is_keyword("DEPTH") {
                    self.advance();
                    max_depth = Some(self.parse_usize()?);
                }
            } else if self.is_keyword("MAX") {
                self.advance();
                self.expect_keyword("DEPTH")?;
                max_depth = Some(self.parse_usize()?);
            } else {
                break;
            }
        }
        Ok(Statement::Iterate {
            bundle,
            start_key,
            step_field,
            max_depth,
        })
    }

    // ── v2.1: Triggers ──

    fn parse_trigger(&mut self, keyword: &str) -> Result<Statement, String> {
        let event_prefix = keyword.to_ascii_uppercase();
        let event_action = self.expect_word()?; // SECTION, REDEFINE, RETRACT, CURVATURE, CONSISTENCY
        let bundle = self.expect_word()?;
        let mut condition = None;
        if self.is_keyword("WHERE") {
            self.advance();
            // Capture condition as raw string
            let mut parts = Vec::new();
            while !self.at_end()
                && !self.is_keyword("EXECUTE")
                && !self.is_keyword("CASCADE")
                && !self.is_keyword("CHECK")
            {
                match self.advance() {
                    Some(Token::Word(w)) => parts.push(w),
                    Some(Token::Number(n)) => parts.push(n.to_string()),
                    Some(Token::Str(s)) => parts.push(format!("'{s}'")),
                    Some(Token::Gt) => parts.push(">".into()),
                    Some(Token::Lt) => parts.push("<".into()),
                    Some(Token::Eq) => parts.push("=".into()),
                    Some(Token::Gte) => parts.push(">=".into()),
                    Some(Token::Lte) => parts.push("<=".into()),
                    Some(Token::Neq) => parts.push("!=".into()),
                    _ => break,
                }
            }
            condition = Some(parts.join(" "));
        }
        // Capture action
        let mut action_parts = Vec::new();
        while !self.at_end() {
            match self.advance() {
                Some(Token::Word(w)) => action_parts.push(w),
                Some(Token::Str(s)) => action_parts.push(format!("'{s}'")),
                Some(Token::Number(n)) => action_parts.push(n.to_string()),
                Some(Token::LParen) => action_parts.push("(".into()),
                Some(Token::RParen) => action_parts.push(")".into()),
                Some(Token::Comma) => action_parts.push(",".into()),
                Some(Token::Eq) => action_parts.push("=".into()),
                Some(Token::Colon) => action_parts.push(":".into()),
                Some(Token::Dot) => action_parts.push(".".into()),
                Some(Token::Star) => action_parts.push("*".into()),
                _ => break,
            }
        }
        let action = action_parts.join(" ");
        let event = format!("{event_prefix} {event_action}");
        Ok(Statement::CreateTrigger {
            event,
            bundle,
            condition,
            action,
        })
    }

    /// Parse: INVALIDATE CACHE [ON <bundle>]
    fn parse_invalidate_cache(&mut self) -> Result<Statement, String> {
        // Expect "CACHE"
        let word = self.expect_word()?;
        if word.to_ascii_uppercase() != "CACHE" {
            return Err(format!("Expected CACHE after INVALIDATE, got: {word}"));
        }
        // Optional: ON <bundle>
        let bundle = if self.is_keyword("ON") {
            self.advance();
            Some(self.expect_word()?)
        } else {
            None
        };
        Ok(Statement::InvalidateCache { bundle })
    }
}

/// Parse a GQL statement string into a Statement AST.
pub fn parse(input: &str) -> Result<Statement, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let stmt = parser.parse()?;
    if matches!(parser.peek(), Some(Token::Semicolon)) {
        parser.advance();
    }
    Ok(stmt)
}

/// Convert a Literal to a GIGI Value.
pub fn literal_to_value(lit: &Literal) -> crate::types::Value {
    match lit {
        Literal::Integer(n) => crate::types::Value::Integer(*n),
        Literal::Float(f) => crate::types::Value::Float(*f),
        Literal::Text(s) => crate::types::Value::Text(s.clone()),
        Literal::Bool(b) => crate::types::Value::Bool(*b),
        Literal::Null => crate::types::Value::Null,
    }
}

/// Convert a FieldSpec to a GIGI FieldDef.
pub fn spec_to_field_def(spec: &FieldSpec) -> crate::types::FieldDef {
    let mut fd = match spec.ftype.to_ascii_uppercase().as_str() {
        "INT" | "INTEGER" | "NUMERIC" => crate::types::FieldDef::numeric(&spec.name),
        "FLOAT" | "REAL" | "DOUBLE" => crate::types::FieldDef::numeric(&spec.name),
        "TEXT" | "VARCHAR" | "STRING" | "CATEGORICAL" => {
            crate::types::FieldDef::categorical(&spec.name)
        }
        "BOOL" | "BOOLEAN" => crate::types::FieldDef::categorical(&spec.name),
        "TIMESTAMP" => crate::types::FieldDef::numeric(&spec.name),
        _ => crate::types::FieldDef::categorical(&spec.name),
    };
    if let Some(r) = spec.range {
        fd = fd.with_range(r);
    }
    if let Some(ref d) = spec.default {
        fd = fd.with_default(literal_to_value(d));
    }
    fd = fd.with_encryption(spec.encryption);
    if let Some(ref g) = spec.encryption_group {
        fd = fd.with_encryption_group(g);
    }
    fd
}

/// Convert an AdjacencySpec (parser AST) to an AdjacencyDef (types).
pub fn adj_spec_to_def(spec: &AdjacencySpec) -> crate::types::AdjacencyDef {
    let kind = match &spec.kind {
        AdjacencySpecKind::Equality { field } => crate::types::AdjacencyKind::Equality {
            field: field.clone(),
        },
        AdjacencySpecKind::Metric { field, radius } => crate::types::AdjacencyKind::Metric {
            field: field.clone(),
            radius: *radius,
        },
        AdjacencySpecKind::Threshold { field, threshold } => {
            crate::types::AdjacencyKind::Threshold {
                field: field.clone(),
                threshold: *threshold,
            }
        }
        AdjacencySpecKind::Transform {
            source_field,
            target_field,
            transform,
        } => {
            let tfn = match transform.to_ascii_lowercase().as_str() {
                "log10" => crate::types::TransformFn::Log10,
                _ => crate::types::TransformFn::Log10, // default fallback; scale/biofilm need args
            };
            crate::types::AdjacencyKind::Transform {
                source_field: source_field.clone(),
                target_field: target_field.clone(),
                transform: tfn,
            }
        }
    };
    crate::types::AdjacencyDef {
        name: spec.name.clone(),
        kind,
        weight: spec.weight,
    }
}

/// Convert a FilterCondition to a QueryCondition.
fn filter_to_query_condition(fc: &FilterCondition) -> crate::bundle::QueryCondition {
    use crate::bundle::QueryCondition as QC;
    match fc {
        FilterCondition::Eq(f, v) => QC::Eq(f.clone(), literal_to_value(v)),
        FilterCondition::Neq(f, v) => QC::Neq(f.clone(), literal_to_value(v)),
        FilterCondition::Gt(f, v) => QC::Gt(f.clone(), literal_to_value(v)),
        FilterCondition::Gte(f, v) => QC::Gte(f.clone(), literal_to_value(v)),
        FilterCondition::Lt(f, v) => QC::Lt(f.clone(), literal_to_value(v)),
        FilterCondition::Lte(f, v) => QC::Lte(f.clone(), literal_to_value(v)),
        FilterCondition::In(f, vs) => QC::In(f.clone(), vs.iter().map(literal_to_value).collect()),
        FilterCondition::NotIn(f, vs) => {
            QC::NotIn(f.clone(), vs.iter().map(literal_to_value).collect())
        }
        FilterCondition::Contains(f, s) => QC::Contains(f.clone(), s.clone()),
        FilterCondition::StartsWith(f, s) => QC::StartsWith(f.clone(), s.clone()),
        FilterCondition::EndsWith(f, s) => QC::EndsWith(f.clone(), s.clone()),
        FilterCondition::Matches(f, s) => QC::Regex(f.clone(), s.clone()),
        FilterCondition::Void(f) => QC::IsNull(f.clone()),
        FilterCondition::Defined(f) => QC::IsNotNull(f.clone()),
        FilterCondition::Between(..) => {
            unreachable!("Between must be desugared via filter_to_query_conditions()")
        }
        // EXISTS is evaluated at a higher level that has access to the engine
        FilterCondition::Exists { .. } => QC::IsNotNull("__always_true__".to_string()),
    }
}

/// Convert FilterCondition::Between into two QueryConditions.
pub fn filter_to_query_conditions(fc: &FilterCondition) -> Vec<crate::bundle::QueryCondition> {
    use crate::bundle::QueryCondition as QC;
    match fc {
        FilterCondition::Between(f, lo, hi) => vec![
            QC::Gte(f.clone(), literal_to_value(lo)),
            QC::Lte(f.clone(), literal_to_value(hi)),
        ],
        FilterCondition::Exists { .. } => vec![], // handled at engine level
        other => vec![filter_to_query_condition(other)],
    }
}

// ── Execution ──

/// Ask G — Phase 2: parsed-and-stored pattern definition that lives in
/// the Engine's in-memory pattern registry. Built from a
/// `Statement::DefinePattern` at execute time; consumed by `Statement::Hunt`.
///
/// Lifetime is tied to the Engine process; lost on restart. Phase 6
/// graduates this into a `gigi_patterns` bundle for persistence + sharing
/// across operators (spec §11 OQ-1).
#[cfg(feature = "patterns")]
#[derive(Debug, Clone)]
pub struct PatternDef {
    pub name: String,
    pub pred: Vec<FilterCondition>,
    pub or_groups: Vec<Vec<FilterCondition>>,
    pub weight: Option<Vec<String>>,
    pub using_fields: Vec<String>,
}

/// Ask G — Phase 3: parsed WEIGHT-expression AST.
///
/// Built once per HUNT invocation from the raw `Vec<String>` tokens that
/// Phase 1 stashes on `PatternDef.weight`, then evaluated against each
/// surviving row to produce the `_score` field. NULL / missing fields
/// coerce to 0.0 per spec §5.3; bool fields coerce to {0.0, 1.0}.
///
/// v0.1 surface: restricted arithmetic over fields and literals.
/// Comparison-in-WEIGHT (`field > threshold`), `CURVATURE(...)` calls,
/// and `CLASSIFY ... WHEN ... THEN ... ELSE` lift in v0.2 (spec §11 OQ-2).
#[cfg(feature = "patterns")]
#[derive(Debug, Clone)]
pub enum WeightExpr {
    Lit(f64),
    Field(String),
    Add(Box<WeightExpr>, Box<WeightExpr>),
    Sub(Box<WeightExpr>, Box<WeightExpr>),
    Mul(Box<WeightExpr>, Box<WeightExpr>),
    Div(Box<WeightExpr>, Box<WeightExpr>),
    /// Two-arg `min(a, b)` — closes the `min(sum, MAX_SCORE)` clip
    /// semantic SCJ named in their 2026-06-09 letter §1.
    Min(Box<WeightExpr>, Box<WeightExpr>),
    /// Two-arg `max(a, b)` — symmetric partner for floor semantics
    /// (e.g. `max(raw_score, 0)`).
    Max(Box<WeightExpr>, Box<WeightExpr>),
}

/// Parse a tokenized WEIGHT body into a WeightExpr AST.
/// Recursive-descent grammar: `expr := term ('+' | '-' term)*`,
/// `term := atom ('*' | '/' atom)*`, `atom := number | ident | '(' expr ')'`.
#[cfg(feature = "patterns")]
fn parse_weight_expr(tokens: &[String]) -> Result<WeightExpr, String> {
    if tokens.is_empty() {
        return Err("Empty WEIGHT expression".to_string());
    }
    let mut pos: usize = 0;
    let expr = parse_weight_add_sub(tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(format!(
            "WEIGHT: unexpected trailing tokens at position {pos}: {:?}",
            &tokens[pos..]
        ));
    }
    Ok(expr)
}

#[cfg(feature = "patterns")]
fn parse_weight_add_sub(tokens: &[String], pos: &mut usize) -> Result<WeightExpr, String> {
    let mut left = parse_weight_mul_div(tokens, pos)?;
    while *pos < tokens.len() {
        let op = tokens[*pos].as_str();
        if op != "+" && op != "-" {
            break;
        }
        *pos += 1;
        let right = parse_weight_mul_div(tokens, pos)?;
        left = match op {
            "+" => WeightExpr::Add(Box::new(left), Box::new(right)),
            "-" => WeightExpr::Sub(Box::new(left), Box::new(right)),
            _ => unreachable!(),
        };
    }
    Ok(left)
}

#[cfg(feature = "patterns")]
fn parse_weight_mul_div(tokens: &[String], pos: &mut usize) -> Result<WeightExpr, String> {
    let mut left = parse_weight_atom(tokens, pos)?;
    while *pos < tokens.len() {
        let op = tokens[*pos].as_str();
        if op != "*" && op != "/" {
            break;
        }
        *pos += 1;
        let right = parse_weight_atom(tokens, pos)?;
        left = match op {
            "*" => WeightExpr::Mul(Box::new(left), Box::new(right)),
            "/" => WeightExpr::Div(Box::new(left), Box::new(right)),
            _ => unreachable!(),
        };
    }
    Ok(left)
}

#[cfg(feature = "patterns")]
fn parse_weight_atom(tokens: &[String], pos: &mut usize) -> Result<WeightExpr, String> {
    if *pos >= tokens.len() {
        return Err("WEIGHT: unexpected end of expression".to_string());
    }
    let tok = tokens[*pos].clone();
    if tok == "(" {
        *pos += 1;
        let inner = parse_weight_add_sub(tokens, pos)?;
        if *pos >= tokens.len() || tokens[*pos] != ")" {
            return Err("WEIGHT: expected ')'".to_string());
        }
        *pos += 1;
        Ok(inner)
    } else if let Ok(n) = tok.parse::<f64>() {
        *pos += 1;
        Ok(WeightExpr::Lit(n))
    } else {
        // Identifier — either a field reference, or a function call if
        // immediately followed by `(`. Two-arg `min` / `max` only in
        // v0.1; variadic / conditional / aggregate functions are
        // deferred (spec OQ-2).
        *pos += 1;
        if *pos < tokens.len() && tokens[*pos] == "(" {
            let fname = tok.to_ascii_lowercase();
            *pos += 1; // consume '('
            let arg1 = parse_weight_add_sub(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != "," {
                return Err(format!(
                    "WEIGHT: `{fname}(` expects two comma-separated args"
                ));
            }
            *pos += 1; // consume ','
            let arg2 = parse_weight_add_sub(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != ")" {
                return Err(format!("WEIGHT: `{fname}(` expects closing ')'"));
            }
            *pos += 1; // consume ')'
            match fname.as_str() {
                "min" => Ok(WeightExpr::Min(Box::new(arg1), Box::new(arg2))),
                "max" => Ok(WeightExpr::Max(Box::new(arg1), Box::new(arg2))),
                other => Err(format!(
                    "WEIGHT: unknown function `{other}` (v0.1 supports `min` and `max` only)"
                )),
            }
        } else {
            Ok(WeightExpr::Field(tok))
        }
    }
}

/// Evaluate a WeightExpr against a Record. NULL / missing → 0.0;
/// Bool → {0.0, 1.0}; Integer → f64; Text/Vector/etc → 0.0.
#[cfg(feature = "patterns")]
pub fn eval_weight(expr: &WeightExpr, row: &crate::types::Record) -> f64 {
    match expr {
        WeightExpr::Lit(n) => *n,
        WeightExpr::Field(name) => match row.get(name) {
            Some(crate::types::Value::Integer(i)) => *i as f64,
            Some(crate::types::Value::Float(f)) => *f,
            Some(crate::types::Value::Bool(b)) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0, // Null / Text / Vector / Binary / missing → 0.0
        },
        WeightExpr::Add(l, r) => eval_weight(l, row) + eval_weight(r, row),
        WeightExpr::Sub(l, r) => eval_weight(l, row) - eval_weight(r, row),
        WeightExpr::Mul(l, r) => eval_weight(l, row) * eval_weight(r, row),
        WeightExpr::Div(l, r) => {
            let denom = eval_weight(r, row);
            if denom == 0.0 {
                f64::NAN
            } else {
                eval_weight(l, row) / denom
            }
        }
        // f64::min / f64::max follow IEEE-754 minNum/maxNum: NaN-propagating
        // only if BOTH operands are NaN, otherwise the non-NaN wins. That
        // matches the "clip semantic" intuition — a NaN sub-expression
        // shouldn't poison the clip floor/ceiling.
        WeightExpr::Min(l, r) => eval_weight(l, row).min(eval_weight(r, row)),
        WeightExpr::Max(l, r) => eval_weight(l, row).max(eval_weight(r, row)),
    }
}

/// Patterns v0.2 Phase PE — decomposition tree for a `WeightExpr`.
///
/// Each variant mirrors `WeightExpr` and carries the per-node contribution
/// to the final score. The root node's `contribution()` MUST equal
/// `eval_weight(expr, row)` for the same `(expr, row)` pair — that
/// invariant is the load-bearing test.
///
/// Wire shape: when serialized, the tree becomes a nested JSON object that
/// TUI / debugger clients render as a per-term breakdown. See
/// `theory/patterns/SPEC_v0.2_VERDICT.md` §6.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone)]
pub enum ExplainNode {
    Lit {
        value: f64,
        contribution: f64,
    },
    Field {
        name: String,
        value: f64,
        contribution: f64,
    },
    Add {
        left: Box<ExplainNode>,
        right: Box<ExplainNode>,
        contribution: f64,
    },
    Sub {
        left: Box<ExplainNode>,
        right: Box<ExplainNode>,
        contribution: f64,
    },
    Mul {
        left: Box<ExplainNode>,
        right: Box<ExplainNode>,
        contribution: f64,
    },
    Div {
        left: Box<ExplainNode>,
        right: Box<ExplainNode>,
        contribution: f64,
    },
    Min {
        left: Box<ExplainNode>,
        right: Box<ExplainNode>,
        /// `"left"` or `"right"` — which branch's value was returned.
        chosen: String,
        /// True iff the cap (right) fired — i.e., the left branch wanted
        /// a higher value than the right allowed. Canonical form
        /// `min(value, cap)`: clipped ↔ value > cap.
        clipped: bool,
        contribution: f64,
    },
    Max {
        left: Box<ExplainNode>,
        right: Box<ExplainNode>,
        chosen: String,
        /// True iff the floor (right) fired — left was below the floor.
        floored: bool,
        contribution: f64,
    },
}

#[cfg(feature = "patterns")]
impl ExplainNode {
    /// The numeric contribution of this node — for the root, equals the
    /// `_score` value `eval_weight` produces.
    pub fn contribution(&self) -> f64 {
        match self {
            ExplainNode::Lit { contribution, .. }
            | ExplainNode::Field { contribution, .. }
            | ExplainNode::Add { contribution, .. }
            | ExplainNode::Sub { contribution, .. }
            | ExplainNode::Mul { contribution, .. }
            | ExplainNode::Div { contribution, .. }
            | ExplainNode::Min { contribution, .. }
            | ExplainNode::Max { contribution, .. } => *contribution,
        }
    }
}

/// Decompose a `WeightExpr` evaluation against a `Record` into a tree of
/// per-node contributions. Patterns v0.2 §6.3.
///
/// Invariant: `explain(expr, row).contribution() == eval_weight(expr, row)`.
/// Verified by `pe6_explain_full_scj_scorer_invariant` and others in
/// `tests/pattern_v02_explain.rs`.
#[cfg(feature = "patterns")]
pub fn explain(expr: &WeightExpr, row: &crate::types::Record) -> ExplainNode {
    match expr {
        WeightExpr::Lit(v) => ExplainNode::Lit {
            value: *v,
            contribution: *v,
        },
        WeightExpr::Field(name) => {
            // Mirror eval_weight's coercion rules so the leaf value matches.
            let value = match row.get(name) {
                Some(crate::types::Value::Integer(i)) => *i as f64,
                Some(crate::types::Value::Float(f)) => *f,
                Some(crate::types::Value::Bool(b)) => {
                    if *b {
                        1.0
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            };
            ExplainNode::Field {
                name: name.clone(),
                value,
                contribution: value,
            }
        }
        WeightExpr::Add(l, r) => {
            let left = explain(l, row);
            let right = explain(r, row);
            let contribution = left.contribution() + right.contribution();
            ExplainNode::Add {
                left: Box::new(left),
                right: Box::new(right),
                contribution,
            }
        }
        WeightExpr::Sub(l, r) => {
            let left = explain(l, row);
            let right = explain(r, row);
            let contribution = left.contribution() - right.contribution();
            ExplainNode::Sub {
                left: Box::new(left),
                right: Box::new(right),
                contribution,
            }
        }
        WeightExpr::Mul(l, r) => {
            let left = explain(l, row);
            let right = explain(r, row);
            let contribution = left.contribution() * right.contribution();
            ExplainNode::Mul {
                left: Box::new(left),
                right: Box::new(right),
                contribution,
            }
        }
        WeightExpr::Div(l, r) => {
            let left = explain(l, row);
            let right = explain(r, row);
            // Mirror eval_weight's NaN-on-zero-denom semantics exactly.
            let denom = right.contribution();
            let contribution = if denom == 0.0 {
                f64::NAN
            } else {
                left.contribution() / denom
            };
            ExplainNode::Div {
                left: Box::new(left),
                right: Box::new(right),
                contribution,
            }
        }
        WeightExpr::Min(l, r) => {
            let left = explain(l, row);
            let right = explain(r, row);
            let lv = left.contribution();
            let rv = right.contribution();
            let contribution = lv.min(rv);
            // Canonical form is `min(value, cap)`. The cap (right) fired
            // when value (left) wanted to be higher.
            let chosen = if lv <= rv { "left" } else { "right" };
            let clipped = lv > rv;
            ExplainNode::Min {
                left: Box::new(left),
                right: Box::new(right),
                chosen: chosen.to_string(),
                clipped,
                contribution,
            }
        }
        WeightExpr::Max(l, r) => {
            let left = explain(l, row);
            let right = explain(r, row);
            let lv = left.contribution();
            let rv = right.contribution();
            let contribution = lv.max(rv);
            let chosen = if lv >= rv { "left" } else { "right" };
            let floored = lv < rv;
            ExplainNode::Max {
                left: Box::new(left),
                right: Box::new(right),
                chosen: chosen.to_string(),
                floored,
                contribution,
            }
        }
    }
}

/// Patterns v0.2 Phase PP — three-layer preflight verdict.
///
/// Per `theory/patterns/SPEC_v0.2_VERDICT.md` §3.1.5 — discovered during
/// Python math validation:
///
///   - `UnsatInternal`: predicate contradicts itself (no bundle can repair).
///     Always a verdict gate.
///   - `UnsatStatistic`: a clause is unsatisfiable given the bundle's
///     field stats. Verdict gate ONLY when `near_miss_budget = 0`.
///   - `UnsatJoint`: each clause individually has support but the joint
///     conjunction has no satisfying row. Informational — used by
///     `verdict()` in the final unsat branch to explain *why*.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone, PartialEq)]
pub enum PreflightVerdict {
    Ok,
    UnsatInternal(String),
    UnsatStatistic(String),
    UnsatJoint(String),
}

#[cfg(feature = "patterns")]
impl PreflightVerdict {
    pub fn is_ok(&self) -> bool {
        matches!(self, PreflightVerdict::Ok)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            PreflightVerdict::Ok => None,
            PreflightVerdict::UnsatInternal(s)
            | PreflightVerdict::UnsatStatistic(s)
            | PreflightVerdict::UnsatJoint(s) => Some(s.as_str()),
        }
    }
}

/// Helper: convert a `Literal` to f64 for numeric range comparison.
/// Returns None for non-numeric literals.
#[cfg(feature = "patterns")]
fn literal_as_f64(lit: &Literal) -> Option<f64> {
    match lit {
        Literal::Integer(i) => Some(*i as f64),
        Literal::Float(f) => Some(*f),
        Literal::Bool(true) => Some(1.0),
        Literal::Bool(false) => Some(0.0),
        Literal::Text(_) | Literal::Null => None,
    }
}

/// Extract `(field, op_kind, numeric_bound)` from a numeric-comparison clause.
/// Returns `None` for non-numeric clauses (Text equalities, Contains, etc.).
///
/// `op_kind` is one of `">="`, `">"`, `"<="`, `"<"`, `"=="`, `"!="`.
#[cfg(feature = "patterns")]
fn numeric_clause_parts(c: &FilterCondition) -> Option<(&str, &'static str, f64)> {
    match c {
        FilterCondition::Eq(f, l) => literal_as_f64(l).map(|v| (f.as_str(), "==", v)),
        FilterCondition::Neq(f, l) => literal_as_f64(l).map(|v| (f.as_str(), "!=", v)),
        FilterCondition::Gt(f, l) => literal_as_f64(l).map(|v| (f.as_str(), ">", v)),
        FilterCondition::Gte(f, l) => literal_as_f64(l).map(|v| (f.as_str(), ">=", v)),
        FilterCondition::Lt(f, l) => literal_as_f64(l).map(|v| (f.as_str(), "<", v)),
        FilterCondition::Lte(f, l) => literal_as_f64(l).map(|v| (f.as_str(), "<=", v)),
        _ => None,
    }
}

/// Pattern preflight layer 1 — internal contradiction check.
///
/// Detects self-contradictory predicates that no bundle row could ever
/// satisfy, regardless of field values:
///
///   - Numeric range contradictions: `x >= 5 AND x < 3`
///   - Equality contradictions: `color == 'red' AND color == 'blue'`
///
/// Always a verdict gate — internal contradictions cannot be repaired
/// by flipping bundle rows.
#[cfg(feature = "patterns")]
pub fn preflight_internal(pred: &[FilterCondition]) -> PreflightVerdict {
    use std::collections::HashMap;

    // Group clauses by field name.
    let mut by_field: HashMap<String, Vec<(&'static str, Option<f64>, &Literal)>> = HashMap::new();
    for c in pred {
        if let Some((field, op, v)) = numeric_clause_parts(c) {
            by_field
                .entry(field.to_string())
                .or_default()
                .push((op, Some(v), lit_ref(c)));
        } else if let FilterCondition::Eq(f, l) = c {
            by_field.entry(f.clone()).or_default().push(("==", None, l));
        } else if let FilterCondition::Neq(f, l) = c {
            by_field.entry(f.clone()).or_default().push(("!=", None, l));
        }
    }

    for (field, ops) in by_field {
        // Numeric lo/hi contradiction
        let ge_lo: Option<f64> = ops
            .iter()
            .filter_map(|(op, v, _)| if matches!(*op, ">=" | ">") { *v } else { None })
            .reduce(f64::max);
        let le_hi: Option<f64> = ops
            .iter()
            .filter_map(|(op, v, _)| if matches!(*op, "<=" | "<") { *v } else { None })
            .reduce(f64::min);
        if let (Some(lo), Some(hi)) = (ge_lo, le_hi) {
            if lo > hi {
                return PreflightVerdict::UnsatInternal(format!(
                    "internal contradiction on {field}: lo={lo}, hi={hi}"
                ));
            }
        }

        // Equality contradiction: two distinct equality literals
        let eqs: Vec<&Literal> = ops
            .iter()
            .filter_map(|(op, _, lit)| if *op == "==" { Some(*lit) } else { None })
            .collect();
        if eqs.len() > 1 {
            let first = eqs[0];
            if eqs.iter().any(|l| !literals_eq(l, first)) {
                return PreflightVerdict::UnsatInternal(format!(
                    "internal contradiction on {field}: == multiple distinct values"
                ));
            }
        }
    }

    PreflightVerdict::Ok
}

#[cfg(feature = "patterns")]
fn lit_ref(c: &FilterCondition) -> &Literal {
    // Helper for preflight_internal — extract literal from comparison ops.
    static NULL_LIT: Literal = Literal::Null;
    match c {
        FilterCondition::Eq(_, l)
        | FilterCondition::Neq(_, l)
        | FilterCondition::Gt(_, l)
        | FilterCondition::Gte(_, l)
        | FilterCondition::Lt(_, l)
        | FilterCondition::Lte(_, l) => l,
        _ => &NULL_LIT,
    }
}

#[cfg(feature = "patterns")]
fn literals_eq(a: &Literal, b: &Literal) -> bool {
    match (a, b) {
        (Literal::Integer(x), Literal::Integer(y)) => x == y,
        (Literal::Float(x), Literal::Float(y)) => x == y,
        (Literal::Text(x), Literal::Text(y)) => x == y,
        (Literal::Bool(x), Literal::Bool(y)) => x == y,
        (Literal::Null, Literal::Null) => true,
        _ => false,
    }
}

/// Pattern preflight layer 2 — bundle-statistic check.
///
/// Detects clauses that the bundle's actual field distribution cannot
/// satisfy (e.g. `x >= 100` against a bundle where `max(x) = 9`).
///
/// Verdict gate ONLY when `near_miss_budget = 0`. With a budget, near-miss
/// may repair what statistic says is impossible, so the scan handles the
/// verdict instead.
///
/// Runs `preflight_internal` first — internal contradictions always win.
#[cfg(feature = "patterns")]
pub fn preflight_statistic(
    pred: &[FilterCondition],
    records: &[crate::types::Record],
) -> PreflightVerdict {
    // Internal contradictions always win.
    let internal = preflight_internal(pred);
    if !internal.is_ok() {
        return internal;
    }

    // For each clause, check whether ANY record in the bundle could satisfy
    // just that clause (single-clause feasibility against actual bundle values).
    let qs: Vec<crate::bundle::QueryCondition> = pred
        .iter()
        .flat_map(filter_to_query_conditions)
        .collect();
    for q in &qs {
        let any_match = records.iter().any(|r| q.matches(r));
        if !any_match {
            return PreflightVerdict::UnsatStatistic(format!(
                "no record satisfies {q:?}"
            ));
        }
    }
    PreflightVerdict::Ok
}

/// Pattern preflight layer 3 — joint-distribution (holonomy) check.
///
/// Detects predicates where each individual clause is satisfiable but
/// the conjunction is empirically forbidden by the bundle's joint
/// distribution. Mirrors SUDOKU's holonomy_preflight from
/// `src/geometry/sudoku.rs` at the predicate level — a non-trivial
/// holonomy loop on the constraint graph corresponds to "no row jointly
/// satisfies all clauses, yet each clause has individual support."
///
/// Informational — `verdict()` uses this in the final unsat branch to
/// give the operator a *why*, but it doesn't pre-empt near-miss detection.
#[cfg(feature = "patterns")]
pub fn preflight_holonomy(
    pred: &[FilterCondition],
    records: &[crate::types::Record],
) -> PreflightVerdict {
    // Multi-field predicates only — single-clause is layer 2's job.
    let fields: std::collections::HashSet<&str> = pred
        .iter()
        .filter_map(|c| match c {
            FilterCondition::Eq(f, _)
            | FilterCondition::Neq(f, _)
            | FilterCondition::Gt(f, _)
            | FilterCondition::Gte(f, _)
            | FilterCondition::Lt(f, _)
            | FilterCondition::Lte(f, _) => Some(f.as_str()),
            _ => None,
        })
        .collect();
    if fields.len() < 2 {
        return PreflightVerdict::Ok;
    }

    // Build full conjunction query from the predicate.
    let qs: Vec<crate::bundle::QueryCondition> = pred
        .iter()
        .flat_map(filter_to_query_conditions)
        .collect();

    // Does ANY row satisfy the full conjunction?
    let any_joint = records
        .iter()
        .any(|r| qs.iter().all(|q| q.matches(r)));
    if any_joint {
        return PreflightVerdict::Ok;
    }

    // Every clause must individually have support — if any clause is
    // already unsat alone, layer 2 catches it, holonomy is silent.
    for q in &qs {
        if !records.iter().any(|r| q.matches(r)) {
            return PreflightVerdict::Ok;
        }
    }

    let field_list: Vec<&str> = fields.into_iter().collect();
    PreflightVerdict::UnsatJoint(format!(
        "joint distribution forbids conjunction across {field_list:?}"
    ))
}

/// Patterns v0.2 Phase VT — HUNT verdict trichotomy.
///
/// Every (pattern, bundle, near_miss_budget) lands in exactly one of:
///
///   - `Sat`: at least one row strictly matches the predicate.
///   - `Unsat`: provably zero matches AND no near-miss within budget.
///     `preflight_caught` flags whether layer-1/2 preflight short-circuited
///     (saving the scan) vs. the scan returning empty + no near-miss.
///   - `NearMiss`: zero strict matches, but ≥1 row is within
///     `near_miss_budget` violations of matching.
///
/// Per spec §4. Companion to `compute_verdict`.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Sat {
        n_matches: usize,
    },
    Unsat {
        reason: String,
        preflight_caught: bool,
    },
    NearMiss {
        near_miss_count: usize,
        budget: usize,
    },
}

/// Compute the verdict trichotomy for a (predicate, bundle, budget) tuple.
///
/// Order, per spec §4.2:
///   1. preflight_internal — always gates. Returns Unsat with preflight_caught.
///   2. If budget == 0: preflight_statistic. Same.
///   3. Sat scan: any row matches strictly → Sat.
///   4. Near-miss scan: any row has 0 < violations ≤ budget → NearMiss.
///   5. preflight_holonomy informational pass → Unsat with reason (joint
///      contradiction explanation if applicable, else plain "no matches").
#[cfg(feature = "patterns")]
pub fn compute_verdict(
    pred: &[FilterCondition],
    records: &[crate::types::Record],
    near_miss_budget: usize,
) -> Verdict {
    // Layer 1: internal contradiction always wins.
    let internal = preflight_internal(pred);
    if let PreflightVerdict::UnsatInternal(reason) = &internal {
        return Verdict::Unsat {
            reason: reason.clone(),
            preflight_caught: true,
        };
    }

    // Layer 2: bundle-statistic preflight only at budget == 0.
    if near_miss_budget == 0 {
        let stat = preflight_statistic(pred, records);
        if let PreflightVerdict::UnsatStatistic(reason) = &stat {
            return Verdict::Unsat {
                reason: reason.clone(),
                preflight_caught: true,
            };
        }
    }

    // Sat scan via QueryCondition::matches (reused from v0.1 path).
    let qs: Vec<crate::bundle::QueryCondition> = pred
        .iter()
        .flat_map(filter_to_query_conditions)
        .collect();
    let n_matches = records
        .iter()
        .filter(|r| qs.iter().all(|q| q.matches(r)))
        .count();
    if n_matches > 0 {
        return Verdict::Sat { n_matches };
    }

    // Near-miss scan. A row is a near-miss iff it has at least one
    // violation but no more than `near_miss_budget` of them.
    if near_miss_budget > 0 {
        let near_miss_count = records
            .iter()
            .filter(|r| {
                let n_violations = qs.iter().filter(|q| !q.matches(r)).count();
                n_violations > 0 && n_violations <= near_miss_budget
            })
            .count();
        if near_miss_count > 0 {
            return Verdict::NearMiss {
                near_miss_count,
                budget: near_miss_budget,
            };
        }
    }

    // True unsat. Run holonomy as an informational pass for the reason.
    let holo = preflight_holonomy(pred, records);
    let reason = match holo {
        PreflightVerdict::UnsatJoint(s) => s,
        _ => "no matches and no near-misses within budget".to_string(),
    };
    Verdict::Unsat {
        reason,
        preflight_caught: false,
    }
}

/// Patterns v0.2 Phase PR — single field flip in a repair sequence.
///
/// `target` is the value the field would need to take to satisfy the
/// corresponding predicate clause. For equality predicates it's the
/// right-hand literal; for range predicates it's a representative
/// satisfying value (the boundary, in v0.2).
#[cfg(feature = "patterns")]
#[derive(Debug, Clone, PartialEq)]
pub struct RepairFlip {
    pub field: String,
    pub current: crate::types::Value,
    pub target: crate::types::Value,
}

/// One option in the repair menu — a list of flips and their total cost.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone, PartialEq)]
pub struct RepairOption {
    pub flips: Vec<RepairFlip>,
    pub cost: f64,
}

/// The repair menu returned by `repair_menu`. Three shapes:
///   - `Options(opts)`: ordered list, opts[0] is the min-cost path.
///   - `AlreadyMatches`: the row already satisfies the predicate.
///   - `TooFar`: the row has more violations than `max_flips` allows.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone, PartialEq)]
pub enum RepairMenu {
    AlreadyMatches,
    TooFar { violations: usize, max_flips: usize },
    Options(Vec<RepairOption>),
}

/// Patterns v0.2 Phase PR — ordered minimum-cost flip sequence.
///
/// For a near-miss row, enumerate the flip sequences that would make
/// it satisfy the predicate, sorted by total cost ascending then by
/// flip count then lexicographic field name.
///
/// Costs default to 1.0 per flip; per-field overrides via `costs`.
/// Result is capped to `top` entries.
///
/// v0.2 only handles equality clauses (right-hand literal is the target).
/// Range / Contains / Matches clauses are silently skipped — repair
/// for those needs a richer target-value enumeration (v0.3).
#[cfg(feature = "patterns")]
pub fn repair_menu(
    pred: &[FilterCondition],
    row: &crate::types::Record,
    max_flips: usize,
    costs: &std::collections::HashMap<String, f64>,
    top: usize,
) -> RepairMenu {
    // Determine which clauses fail and what target each one wants.
    let qs: Vec<crate::bundle::QueryCondition> = pred
        .iter()
        .flat_map(filter_to_query_conditions)
        .collect();
    let violating_qs: Vec<&crate::bundle::QueryCondition> =
        qs.iter().filter(|q| !q.matches(row)).collect();

    if violating_qs.is_empty() {
        return RepairMenu::AlreadyMatches;
    }
    if violating_qs.len() > max_flips {
        return RepairMenu::TooFar {
            violations: violating_qs.len(),
            max_flips,
        };
    }

    // Collect each violating clause's (field, target) where possible.
    // v0.2: equality clauses only. The full enumeration over satisfying
    // values for range clauses is deferred to v0.3.
    let mut flip_specs: Vec<(String, crate::types::Value, crate::types::Value)> = Vec::new();
    for q in &violating_qs {
        if let crate::bundle::QueryCondition::Eq(field, target_val) = q {
            let current = row
                .get(field)
                .cloned()
                .unwrap_or(crate::types::Value::Null);
            flip_specs.push((field.clone(), current, target_val.clone()));
        }
    }
    if flip_specs.len() < violating_qs.len() {
        // Some violations cannot be expressed as a single flip in v0.2.
        return RepairMenu::TooFar {
            violations: violating_qs.len(),
            max_flips,
        };
    }

    // For v0.2 (equality-only), each violating clause has exactly one
    // target value. So the canonical full-repair sequence is unique:
    // flip every violating clause. There may be sub-sequences if
    // max_flips > violations, but those wouldn't satisfy the predicate.
    let total_cost: f64 = flip_specs
        .iter()
        .map(|(f, _, _)| costs.get(f).copied().unwrap_or(1.0))
        .sum();

    let mut sorted = flip_specs.clone();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let flips: Vec<RepairFlip> = sorted
        .into_iter()
        .map(|(field, current, target)| RepairFlip {
            field,
            current,
            target,
        })
        .collect();

    let option = RepairOption {
        flips,
        cost: total_cost,
    };
    let mut opts = vec![option];
    if opts.len() > top {
        opts.truncate(top);
    }
    RepairMenu::Options(opts)
}

/// Patterns v0.2 Phase K_P — pattern curvature report.
///
/// Per-bundle scalar derived from the variance of the per-row
/// neighbor-match ratio in the kNN graph over the `using` fields.
///
/// Interpretation:
///   - `k_p > 0` with low n_matches: pattern picks out a localized region
///   - `k_p == 0` with high n_matches: tautology (every neighborhood
///      has 100% match rate)
///   - `k_p == 0` with n_matches == 0: empty match set (convention)
#[cfg(feature = "patterns")]
#[derive(Debug, Clone, PartialEq)]
pub struct PatternCurvature {
    pub k_p: f64,
    pub n_matches: usize,
    pub n_rows: usize,
    pub k: usize,
}

/// Hamming distance between two records over a named field set.
/// Missing fields on either side count as a mismatch.
#[cfg(feature = "patterns")]
fn record_hamming(a: &crate::types::Record, b: &crate::types::Record, fields: &[String]) -> usize {
    fields
        .iter()
        .filter(|f| a.get(f.as_str()) != b.get(f.as_str()))
        .count()
}

/// Patterns v0.2 Phase K_P — compute pattern curvature.
///
/// Algorithm:
///   1. Find the match set (indices of rows the predicate strictly satisfies).
///   2. For each row, compute its kNN by Hamming distance over `fields`.
///   3. For each row, compute the fraction of its neighbors in the match set.
///   4. K_P = sample variance of those fractions.
///
/// Complexity: O(N² · |fields|) for the naive kNN scan. For Marcella-scale
/// bundles (~10k rows) that's fine; for larger bundles the kNN should
/// be precomputed and cached via the existing BundleStats infrastructure.
///
/// Returns (0.0, 0) when n_matches == 0 by convention.
#[cfg(feature = "patterns")]
pub fn pattern_curvature(
    pred: &[FilterCondition],
    records: &[crate::types::Record],
    fields: &[String],
    k: usize,
) -> PatternCurvature {
    let n_rows = records.len();
    let qs: Vec<crate::bundle::QueryCondition> = pred
        .iter()
        .flat_map(filter_to_query_conditions)
        .collect();
    let match_set: std::collections::HashSet<usize> = (0..n_rows)
        .filter(|i| qs.iter().all(|q| q.matches(&records[*i])))
        .collect();
    let n_matches = match_set.len();
    if n_matches == 0 || n_rows == 0 {
        return PatternCurvature {
            k_p: 0.0,
            n_matches: 0,
            n_rows,
            k,
        };
    }

    // For each row, compute its k nearest neighbors by Hamming distance
    // (with the row's own index excluded).
    let k_eff = k.min(n_rows.saturating_sub(1));
    let mut ratios = Vec::with_capacity(n_rows);
    let mut distances: Vec<(usize, usize)> = Vec::with_capacity(n_rows);
    for i in 0..n_rows {
        distances.clear();
        for j in 0..n_rows {
            if i == j {
                continue;
            }
            distances.push((j, record_hamming(&records[i], &records[j], fields)));
        }
        distances.sort_by_key(|t| t.1);
        let matches_in_nbrs = distances
            .iter()
            .take(k_eff)
            .filter(|(j, _)| match_set.contains(j))
            .count();
        let ratio = matches_in_nbrs as f64 / k_eff.max(1) as f64;
        ratios.push(ratio);
    }
    let mean = ratios.iter().copied().sum::<f64>() / ratios.len() as f64;
    let variance = ratios
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / ratios.len() as f64;

    PatternCurvature {
        k_p: variance,
        n_matches,
        n_rows,
        k: k_eff,
    }
}

/// Patterns v0.2 wire — HUNT request shape (library-level, mirrors the HTTP body).
///
/// Composed from the v0.1 fields plus the four v0.2 flags. Use
/// `near_miss_budget = 0` + all booleans false for v0.1-compatible behavior.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone)]
pub struct HuntV2Args {
    pub pattern: String,
    pub bundle: String,
    pub excluding: Vec<String>,
    pub top: Option<usize>,
    pub project: Option<Vec<String>>,
    pub near_miss_budget: usize,
    pub explain: bool,
    pub include_repair_menu: bool,
    pub relaxation_costs: std::collections::HashMap<String, f64>,
}

/// One near-miss row paired with its repair menu (if requested).
#[cfg(feature = "patterns")]
#[derive(Debug, Clone)]
pub struct NearMissRow {
    pub row: crate::types::Record,
    pub repair_menu: Option<RepairMenu>,
}

/// HUNT v0.2 response envelope.
///
/// One struct, three verdicts, optional sat/near-miss/unsat fields populated
/// per verdict. Designed to serialize to the JSON wire shape per
/// `theory/patterns/SPEC_v0.2_VERDICT.md` §4.
#[cfg(feature = "patterns")]
#[derive(Debug, Clone)]
pub struct HuntV2Envelope {
    /// `"sat" | "near_miss" | "unsat"`.
    pub verdict: String,
    /// Sat: top-K scored rows. NearMiss/Unsat: empty.
    pub rows: Vec<crate::types::Record>,
    pub n_matches: Option<usize>,
    pub near_miss_count: Option<usize>,
    /// Populated when verdict == near_miss. Each row carries its
    /// `_repair_menu` JSON field when `include_repair_menu` was set.
    pub near_miss_rows: Vec<NearMissRow>,
    pub reason: Option<String>,
    pub preflight_caught: Option<bool>,
}

/// Orchestrate a Patterns v0.2 HUNT into a single envelope.
///
/// Composes all five v0.2 primitives:
///   - PP preflight (via compute_verdict)
///   - VT verdict trichotomy
///   - PE explain (when `args.explain`)
///   - PR repair menu (when `args.include_repair_menu`)
///   - K_P curvature (NOT included by default; opt-in via separate endpoint)
///
/// Branches by verdict:
///   - **Sat**: runs the v0.1 HUNT pipeline (predicate → WEIGHT eval → sort
///     → top-N → project). Optionally attaches `_explain` to each row.
///   - **NearMiss**: scans for rows with 0 < violations ≤ budget. Optionally
///     attaches `_repair_menu` to each. Applies top + project to the
///     near-miss rows.
///   - **Unsat**: empty rows + reason + preflight_caught flag.
#[cfg(feature = "patterns")]
pub fn hunt_v2_orchestrate(
    engine: &mut crate::engine::Engine,
    args: &HuntV2Args,
) -> Result<HuntV2Envelope, String> {
    // 1. Look up pattern in registry.
    let pat = engine
        .pattern_registry
        .get(&args.pattern)
        .ok_or_else(|| format!("Pattern '{}' is not defined.", args.pattern))?
        .clone();
    if engine.bundle(&args.bundle).is_none() {
        return Err(format!("Bundle '{}' does not exist.", args.bundle));
    }

    // 2. Get full record set, post-exclusion. Use the existing COVER path
    //    (no predicate) so EXCLUDING IN composes the same way v0.1 does.
    let all_cover = Statement::Cover {
        bundle: args.bundle.clone(),
        on_conditions: Vec::new(),
        where_conditions: Vec::new(),
        or_groups: Vec::new(),
        distinct_field: None,
        project: None,
        rank_by: None,
        first: None,
        skip: None,
        all: true,
        excluding: args.excluding.clone(),
    };
    let all_records: Vec<crate::types::Record> = match execute(engine, &all_cover)? {
        ExecResult::Rows(rs) => rs,
        other => {
            return Err(format!(
                "internal: COVER did not return rows: {other:?}"
            ))
        }
    };

    // 3. Compute verdict over the filtered record set.
    let verdict = compute_verdict(&pat.pred, &all_records, args.near_miss_budget);

    match verdict {
        Verdict::Sat { n_matches } => {
            // Run the v0.1 HUNT pipeline to get scored, sorted, top-N rows.
            let mut hunt_sql = format!("HUNT {} IN {}", args.pattern, args.bundle);
            for excl in &args.excluding {
                hunt_sql.push_str(" EXCLUDING IN ");
                hunt_sql.push_str(excl);
            }
            if let Some(n) = args.top {
                hunt_sql.push_str(&format!(" TOP {n}"));
            }
            if let Some(proj) = &args.project {
                hunt_sql.push_str(" PROJECT (");
                hunt_sql.push_str(&proj.join(", "));
                hunt_sql.push(')');
            }
            let stmt = parse(&hunt_sql).map_err(|e| format!("hunt sql parse: {e}"))?;
            let mut rows = match execute(engine, &stmt)? {
                ExecResult::Rows(rs) => rs,
                other => return Err(format!("HUNT did not return rows: {other:?}")),
            };

            // Optional: attach _explain to each row.
            if args.explain {
                let weight_expr: Option<WeightExpr> = match &pat.weight {
                    Some(toks) => Some(parse_weight_expr(toks)?),
                    None => None,
                };
                if let Some(expr) = weight_expr {
                    for row in &mut rows {
                        let node = explain(&expr, row);
                        let json = serde_json::to_string(&explain_node_to_json(&node))
                            .map_err(|e| format!("explain serialize: {e}"))?;
                        row.insert("_explain".to_string(), crate::types::Value::Text(json));
                    }
                }
            }

            Ok(HuntV2Envelope {
                verdict: "sat".to_string(),
                rows,
                n_matches: Some(n_matches),
                near_miss_count: None,
                near_miss_rows: Vec::new(),
                reason: None,
                preflight_caught: None,
            })
        }
        Verdict::NearMiss {
            near_miss_count,
            budget,
        } => {
            // Scan for near-miss rows from the already-filtered record set.
            let qs: Vec<crate::bundle::QueryCondition> = pat
                .pred
                .iter()
                .flat_map(filter_to_query_conditions)
                .collect();
            let mut near_miss_rows: Vec<NearMissRow> = all_records
                .iter()
                .filter_map(|r| {
                    let n_viol = qs.iter().filter(|q| !q.matches(r)).count();
                    if n_viol > 0 && n_viol <= budget {
                        let repair_menu = if args.include_repair_menu {
                            Some(repair_menu(
                                &pat.pred,
                                r,
                                budget,
                                &args.relaxation_costs,
                                5,
                            ))
                        } else {
                            None
                        };
                        let mut row = r.clone();
                        if let Some(menu) = &repair_menu {
                            let json = serde_json::to_string(&repair_menu_to_json(menu))
                                .unwrap_or_else(|_| "null".to_string());
                            row.insert("_repair_menu".to_string(), crate::types::Value::Text(json));
                        }
                        Some(NearMissRow { row, repair_menu })
                    } else {
                        None
                    }
                })
                .collect();

            // Apply top.
            if let Some(n) = args.top {
                near_miss_rows.truncate(n);
            }
            // Apply project (to the row inside each NearMissRow).
            if let Some(proj) = &args.project {
                for nm in &mut near_miss_rows {
                    let mut projected = crate::types::Record::new();
                    for key in proj {
                        if let Some(v) = nm.row.get(key) {
                            projected.insert(key.clone(), v.clone());
                        }
                    }
                    nm.row = projected;
                }
            }

            Ok(HuntV2Envelope {
                verdict: "near_miss".to_string(),
                rows: Vec::new(),
                n_matches: None,
                near_miss_count: Some(near_miss_count),
                near_miss_rows,
                reason: None,
                preflight_caught: None,
            })
        }
        Verdict::Unsat {
            reason,
            preflight_caught,
        } => Ok(HuntV2Envelope {
            verdict: "unsat".to_string(),
            rows: Vec::new(),
            n_matches: None,
            near_miss_count: None,
            near_miss_rows: Vec::new(),
            reason: Some(reason),
            preflight_caught: Some(preflight_caught),
        }),
    }
}

/// Helper: convert an ExplainNode to serde_json::Value for wire serialization.
#[cfg(feature = "patterns")]
pub fn explain_node_to_json(node: &ExplainNode) -> serde_json::Value {
    use serde_json::json;
    match node {
        ExplainNode::Lit { value, contribution } => {
            json!({ "type": "lit", "value": value, "contribution": contribution })
        }
        ExplainNode::Field { name, value, contribution } => {
            json!({ "type": "field", "name": name, "value": value, "contribution": contribution })
        }
        ExplainNode::Add { left, right, contribution } => json!({
            "type": "add",
            "left": explain_node_to_json(left),
            "right": explain_node_to_json(right),
            "contribution": contribution,
        }),
        ExplainNode::Sub { left, right, contribution } => json!({
            "type": "sub",
            "left": explain_node_to_json(left),
            "right": explain_node_to_json(right),
            "contribution": contribution,
        }),
        ExplainNode::Mul { left, right, contribution } => json!({
            "type": "mul",
            "left": explain_node_to_json(left),
            "right": explain_node_to_json(right),
            "contribution": contribution,
        }),
        ExplainNode::Div { left, right, contribution } => json!({
            "type": "div",
            "left": explain_node_to_json(left),
            "right": explain_node_to_json(right),
            "contribution": contribution,
        }),
        ExplainNode::Min { left, right, chosen, clipped, contribution } => json!({
            "type": "min",
            "left": explain_node_to_json(left),
            "right": explain_node_to_json(right),
            "chosen": chosen,
            "clipped": clipped,
            "contribution": contribution,
        }),
        ExplainNode::Max { left, right, chosen, floored, contribution } => json!({
            "type": "max",
            "left": explain_node_to_json(left),
            "right": explain_node_to_json(right),
            "chosen": chosen,
            "floored": floored,
            "contribution": contribution,
        }),
    }
}

/// Helper: convert a RepairMenu to serde_json::Value for wire serialization.
#[cfg(feature = "patterns")]
pub fn repair_menu_to_json(menu: &RepairMenu) -> serde_json::Value {
    use serde_json::json;
    match menu {
        RepairMenu::AlreadyMatches => json!({ "kind": "already_matches" }),
        RepairMenu::TooFar { violations, max_flips } => json!({
            "kind": "too_far",
            "violations": violations,
            "max_flips": max_flips,
        }),
        RepairMenu::Options(opts) => json!({
            "kind": "options",
            "options": opts.iter().map(|o| {
                json!({
                    "cost": o.cost,
                    "flips": o.flips.iter().map(|f| json!({
                        "field": f.field,
                        "current": value_to_json_basic(&f.current),
                        "target": value_to_json_basic(&f.target),
                    })).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        }),
    }
}

#[cfg(feature = "patterns")]
fn value_to_json_basic(v: &crate::types::Value) -> serde_json::Value {
    use crate::types::Value;
    use serde_json::json;
    match v {
        Value::Integer(i) => json!(i),
        Value::Float(f) if f.is_finite() => json!(f),
        Value::Float(_) => serde_json::Value::Null,
        Value::Text(s) => json!(s),
        Value::Bool(b) => json!(b),
        Value::Timestamp(t) => json!(t),
        Value::Vector(v) => json!(v),
        Value::Binary(b) => json!(b),
        Value::Null => serde_json::Value::Null,
    }
}

/// Test helper: parse a complete `DEFINE PATTERN ... WEIGHT (...)` SQL string
/// and return its `WeightExpr`. Convenience wrapper so tests don't have to
/// hand-build the AST for large expressions like the SCJ 10-weight scorer.
///
/// Errors if the SQL doesn't parse as a `DefinePattern` or has no WEIGHT clause.
#[cfg(feature = "patterns")]
pub fn parse_weight_expr_for_test(sql: &str) -> Result<WeightExpr, String> {
    let stmt = parse(sql)?;
    match stmt {
        Statement::DefinePattern { weight: Some(toks), .. } => parse_weight_expr(&toks),
        Statement::DefinePattern { weight: None, .. } => {
            Err("DEFINE PATTERN has no WEIGHT clause".to_string())
        }
        other => Err(format!("expected DefinePattern, got {other:?}")),
    }
}

/// Render a Value to a canonical String key, suitable for hashing in
/// the EXCLUDING IN anti-join set. Each Value variant gets a unique
/// prefix so `Integer(1)` and `Float(1.0)` and `Text("1")` are distinct
/// — PK semantics demand exact-value equality, not numeric coercion.
///
/// Returns None for Null / Vector / Binary (those aren't valid PK types
/// in this engine).
#[cfg(feature = "patterns")]
fn pk_key(v: &crate::types::Value) -> Option<String> {
    use crate::types::Value;
    match v {
        Value::Integer(i) => Some(format!("i{i}")),
        Value::Float(f) => Some(format!("f{f}")),
        Value::Text(s) => Some(format!("t{s}")),
        Value::Bool(b) => Some(format!("b{b}")),
        Value::Timestamp(t) => Some(format!("ts{t}")),
        Value::Null | Value::Vector(_) | Value::Binary(_) => None,
    }
}

/// Apply an `EXCLUDING IN <bundle>...` anti-join to a row set in place.
///
/// For each excluded bundle, validates it exists, fetches all rows via a
/// no-WHERE Cover, extracts the base PK value from each, and accumulates
/// into a HashSet keyed by `pk_key()`. After the union exclusion set is
/// built, retains only rows whose `target_pk_field` value is NOT in the
/// set.
///
/// Used by both HUNT (Phase 4) and COVER (Phase 4 PH15 follow-up). Same
/// PK-only semantics: the excluded bundle's fiber is never read, so
/// schema mismatches between target and exclusion are harmless.
///
/// `target_pk_field == None` (rare: target bundle has no base field on
/// the heap side, e.g. fully mmap'd) is a no-op — we can't filter
/// without a PK to compare. The retain is skipped and rows pass through.
#[cfg(feature = "patterns")]
fn apply_excluding_in_filter(
    engine: &mut crate::engine::Engine,
    excluding: &[String],
    target_pk_field: &Option<String>,
    rows: &mut Vec<crate::types::Record>,
) -> Result<(), String> {
    if excluding.is_empty() {
        return Ok(());
    }
    let mut exclusion_pks: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for excl_name in excluding {
        if engine.bundle(excl_name).is_none() {
            return Err(format!(
                "EXCLUDING IN bundle '{excl_name}' does not exist."
            ));
        }
        let excl_pk_field: String = engine
            .heap_bundle(excl_name)
            .and_then(|s| s.schema.base_fields.first().map(|f| f.name.clone()))
            .unwrap_or_default();
        let excl_cover = Statement::Cover {
            bundle: excl_name.clone(),
            on_conditions: Vec::new(),
            where_conditions: Vec::new(),
            or_groups: Vec::new(),
            distinct_field: None,
            project: None,
            rank_by: None,
            first: None,
            skip: None,
            all: true,
            excluding: Vec::new(),
        };
        let excl_rows = match execute(engine, &excl_cover)? {
            ExecResult::Rows(rs) => rs,
            other => {
                return Err(format!(
                    "EXCLUDING IN inner Cover on '{excl_name}' returned non-Rows: {other:?}"
                ));
            }
        };
        for row in &excl_rows {
            if let Some(v) = row.get(&excl_pk_field) {
                if let Some(k) = pk_key(v) {
                    exclusion_pks.insert(k);
                }
            }
        }
    }
    if let Some(pk_field) = target_pk_field {
        rows.retain(|row| {
            if let Some(v) = row.get(pk_field) {
                if let Some(k) = pk_key(v) {
                    !exclusion_pks.contains(&k)
                } else {
                    true
                }
            } else {
                true
            }
        });
    }
    Ok(())
}

/// Coerce a Value to f64 for sort-key comparison. Returns None when the
/// value can't be reduced to a number.
#[cfg(feature = "patterns")]
fn value_to_f64(v: &crate::types::Value) -> Option<f64> {
    match v {
        crate::types::Value::Integer(i) => Some(*i as f64),
        crate::types::Value::Float(f) => Some(*f),
        crate::types::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Execution result.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecResult {
    Ok,
    Rows(Vec<crate::types::Record>),
    Scalar(f64),
    Bool(bool),
    Count(usize),
    Stats(GqlStats),
    Bundles(Vec<GqlBundleInfo>),
    /// Sprint H: labeled gauge-invariant results. Each entry is
    /// (canonical_label, value). The invariant evaluator guarantees no
    /// decryption is performed during evaluation; see
    /// `crate::invariant::evaluate`.
    Invariants(Vec<(String, f64)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GqlStats {
    pub curvature: f64,
    pub confidence: f64,
    pub record_count: usize,
    pub storage_mode: String,
    pub base_fields: usize,
    pub fiber_fields: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GqlBundleInfo {
    pub name: String,
    pub records: usize,
    pub fields: usize,
}

/// Resolve an `EncryptionSeedSource` to a 32-byte master seed at bundle-creation
/// time. v0.2 — wraps the v0.1 random path with hex / env-var alternatives.
fn resolve_seed(
    source: &crate::types::EncryptionSeedSource,
) -> Result<[u8; 32], String> {
    use crate::types::EncryptionSeedSource as S;
    match source {
        S::Random => Ok(crate::crypto::GaugeKey::random_seed()),
        S::Hex(hex) => crate::crypto::seed_from_hex(hex),
        S::Env(name) => {
            let value = std::env::var(name).map_err(|_| {
                format!(
                    "WITH ENCRYPTION SEED FROM ENV {name}: env var is not set on this engine"
                )
            })?;
            crate::crypto::seed_from_hex(&value).map_err(|e| {
                format!("env var {name} did not contain a valid 64-char hex seed: {e}")
            })
        }
    }
}

/// Execute a parsed statement against an Engine.
pub fn execute(engine: &mut crate::engine::Engine, stmt: &Statement) -> Result<ExecResult, String> {
    match stmt {
        // ── Schema ──
        Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
            invariants,
            seed_source,
        } => {
            let mut schema = crate::types::BundleSchema::new(name);
            for f in base_fields {
                schema = schema.base(spec_to_field_def(f));
            }
            for f in fiber_fields {
                schema = schema.fiber(spec_to_field_def(f));
            }
            for idx in indexed {
                schema = schema.index(idx);
            }
            for adj in adjacencies {
                schema = schema.adjacency(adj_spec_to_def(adj));
            }
            for inv in invariants {
                schema = schema.with_invariant(crate::types::InvariantDef {
                    expr_field: inv.field.clone(),
                    expected: inv.expected,
                    tol: inv.tol,
                });
            }
            // v0.1 backwards-compat: if the bundle-level ENCRYPTED flag is
            // set, propagate type-default modes to every fiber field that
            // doesn't already have an explicit per-field mode (v0.2 syntax).
            // This means `CREATE BUNDLE foo (..) ENCRYPTED` keeps doing what
            // it did in v0.1 (numeric → Affine, others → Opaque) while
            // mixed-mode v0.2 schemas honor the per-field declarations.
            if *encrypted {
                for fd in schema.fiber_fields.iter_mut() {
                    if fd.encryption == crate::types::EncryptionMode::None {
                        fd.encryption = crate::types::EncryptionMode::default_for_type(&fd.field_type);
                    }
                }
                let seed = resolve_seed(seed_source)?;
                let gk = crate::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
                schema.gauge_key = Some(gk);
            } else if schema.fiber_fields.iter().any(|fd| fd.encryption.is_encrypted()) {
                // Per-field encryption declared without bundle-level shorthand.
                // Generate a seed and derive the GaugeKey so the engine has
                // crypto material on hand for fields that ARE encrypted.
                let seed = resolve_seed(seed_source)?;
                let gk = crate::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
                schema.gauge_key = Some(gk);
            }
            engine.create_bundle(schema).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::Collapse { bundle } => {
            engine.drop_bundle(bundle).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::ShowBundles => {
            let infos: Vec<GqlBundleInfo> = engine
                .bundle_names()
                .iter()
                .map(|name| {
                    let store = engine.bundle(name).unwrap();
                    GqlBundleInfo {
                        name: name.to_string(),
                        records: store.len(),
                        fields: store.schema().base_fields.len() + store.schema().fiber_fields.len(),
                    }
                })
                .collect();
            Ok(ExecResult::Bundles(infos))
        }

        Statement::Describe { bundle, verbose: _ } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let stats = store.curvature_stats();
            let k = stats.mean();
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: crate::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema().base_fields.len(),
                fiber_fields: store.schema().fiber_fields.len(),
            }))
        }

        // ── Write ──
        Statement::Insert {
            bundle,
            columns,
            values,
        } => {
            if !columns.is_empty() && columns.len() != values.len() {
                return Err("Column count doesn't match value count".into());
            }
            let mut record = HashMap::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                record.insert(col.clone(), literal_to_value(val));
            }
            engine.insert(bundle, &record).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::BatchInsert {
            bundle,
            columns,
            rows,
        } => {
            let records: Vec<crate::types::Record> = rows
                .iter()
                .map(|row| {
                    if columns.is_empty() {
                        // Positional — use schema field order
                        row.iter()
                            .enumerate()
                            .map(|(i, v)| (format!("_{i}"), literal_to_value(v)))
                            .collect()
                    } else {
                        columns
                            .iter()
                            .zip(row.iter())
                            .map(|(c, v)| (c.clone(), literal_to_value(v)))
                            .collect()
                    }
                })
                .collect();
            engine
                .batch_insert(bundle, &records)
                .map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::BatchSectionUpsert {
            bundle,
            columns,
            rows,
        } => {
            let mut inserted = 0usize;
            let mut updated = 0usize;
            for row in rows {
                let record: crate::types::Record = if columns.is_empty() {
                    row.iter()
                        .enumerate()
                        .map(|(i, v)| (format!("_{i}"), literal_to_value(v)))
                        .collect()
                } else {
                    columns.iter().zip(row.iter())
                        .map(|(c, v)| (c.clone(), literal_to_value(v)))
                        .collect()
                };
                let mut store = engine
                    .bundle_mut(bundle)
                    .ok_or_else(|| format!("No bundle: {bundle}"))?;
                if store.upsert(&record) { updated += 1; } else { inserted += 1; }
            }
            Ok(ExecResult::Scalar(inserted as f64 + updated as f64))
        }

        Statement::SectionUpsert {
            bundle,
            columns,
            values,
        } => {
            let mut record = HashMap::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                record.insert(col.clone(), literal_to_value(val));
            }
            let mut store = engine
                .bundle_mut(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            store.upsert(&record);
            Ok(ExecResult::Ok)
        }

        Statement::Redefine { bundle, key, sets } => {
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let patches: crate::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let updated = engine
                .update(bundle, &key_rec, &patches)
                .map_err(|e| format!("{e}"))?;
            if updated {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }

        Statement::BulkRedefine {
            bundle,
            conditions,
            sets,
        } => {
            let qcs: Vec<crate::bundle::QueryCondition> = conditions
                .iter()
                .flat_map(filter_to_query_conditions)
                .collect();
            let patches: crate::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let mut store = engine
                .bundle_mut(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let matched = store.bulk_update(&qcs, &patches);
            Ok(ExecResult::Count(matched))
        }

        Statement::Retract { bundle, key } => {
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let deleted = engine
                .delete(bundle, &key_rec)
                .map_err(|e| format!("{e}"))?;
            if deleted {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }

        Statement::BulkRetract { bundle, conditions } => {
            let qcs: Vec<crate::bundle::QueryCondition> = conditions
                .iter()
                .flat_map(filter_to_query_conditions)
                .collect();
            let mut store = engine
                .bundle_mut(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let deleted = store.bulk_delete(&qcs);
            Ok(ExecResult::Count(deleted))
        }

        // ── Point Query ──
        Statement::PointQuery {
            bundle,
            key,
            project,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            match store.point_query(&key_rec) {
                Some(mut rec) => {
                    if let Some(fields) = project {
                        rec.retain(|k, _| fields.contains(k));
                    }
                    Ok(ExecResult::Rows(vec![rec]))
                }
                None => Ok(ExecResult::Rows(vec![])),
            }
        }

        Statement::ExistsSection { bundle, key } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            Ok(ExecResult::Bool(store.point_query(&key_rec).is_some()))
        }

        // ── Cover/Range Query ──
        Statement::Cover {
            bundle,
            on_conditions,
            where_conditions,
            or_groups,
            distinct_field,
            project,
            rank_by,
            first,
            skip,
            all: _,
            #[cfg(feature = "patterns")]
            excluding,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;

            // Handle DISTINCT
            if let Some(field) = distinct_field {
                let vals = store.distinct(field);
                let rows: Vec<crate::types::Record> = vals
                    .into_iter()
                    .map(|v| {
                        let mut r = HashMap::new();
                        r.insert(field.clone(), v);
                        r
                    })
                    .collect();
                return Ok(ExecResult::Rows(rows));
            }

            // Build conditions
            let mut conditions: Vec<crate::bundle::QueryCondition> = Vec::new();
            for fc in on_conditions.iter().chain(where_conditions.iter()) {
                conditions.extend(filter_to_query_conditions(fc));
            }

            let or_qcs: Vec<Vec<crate::bundle::QueryCondition>> = or_groups
                .iter()
                .map(|group| group.iter().flat_map(filter_to_query_conditions).collect())
                .collect();

            let or_ref = if or_qcs.is_empty() {
                None
            } else {
                Some(or_qcs.as_slice())
            };

            // PH15: same defer trick — when EXCLUDING IN is present, the
            // anti-join must see the full result set (after RANK, before
            // SKIP/FIRST) so the FIRST budget isn't spent on rows we're
            // about to drop. NOTE: if PROJECT omits the base PK, the
            // post-projection EXCLUDING IN can't match. v0.1 contract:
            // include the PK in PROJECT (or omit PROJECT) when using
            // EXCLUDING IN. Documented in spec §6.
            #[cfg(feature = "patterns")]
            let (eff_first_p, eff_skip_p) = if !excluding.is_empty() {
                (None, None)
            } else {
                (*first, *skip)
            };
            #[cfg(not(feature = "patterns"))]
            let (eff_first_p, eff_skip_p) = (*first, *skip);

            // Use projected query if PROJECT specified
            let results = if let Some(fields) = project {
                let sort_refs: Vec<(&str, bool)> = rank_by
                    .as_ref()
                    .map(|specs| specs.iter().map(|s| (s.field.as_str(), s.desc)).collect())
                    .unwrap_or_default();
                let sort_opt = if sort_refs.is_empty() {
                    None
                } else {
                    Some(sort_refs.as_slice())
                };
                let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
                let (rows, _total) = store.filtered_query_projected_ex(
                    &conditions,
                    or_ref,
                    sort_opt,
                    eff_first_p,
                    eff_skip_p,
                    Some(&field_refs),
                );
                rows
            } else {
                // Use simple filtered_query_ex with single sort field
                let (sort_by, sort_desc) = rank_by
                    .as_ref()
                    .and_then(|specs| specs.first())
                    .map(|s| (Some(s.field.as_str()), s.desc))
                    .unwrap_or((None, false));
                store.filtered_query_ex(&conditions, or_ref, sort_by, sort_desc, eff_first_p, eff_skip_p)
            };

            // ── PH15 — apply EXCLUDING IN anti-join (Ask G follow-up) ──
            // Reuses the same helper HUNT uses. PK-only access; bundle
            // schema mismatches between target and exclusion are
            // harmless. Empty `excluding` short-circuits the helper.
            #[cfg(feature = "patterns")]
            let mut results = results;
            #[cfg(feature = "patterns")]
            {
                let target_pk_field: Option<String> = engine
                    .heap_bundle(bundle)
                    .and_then(|s| s.schema.base_fields.first().map(|f| f.name.clone()));
                apply_excluding_in_filter(engine, excluding, &target_pk_field, &mut results)?;

                // Apply the deferred SKIP + FIRST after the anti-join.
                if !excluding.is_empty() {
                    if let Some(n) = skip {
                        if *n < results.len() {
                            results.drain(0..*n);
                        } else {
                            results.clear();
                        }
                    }
                    if let Some(n) = first {
                        results.truncate(*n);
                    }
                }
            }

            Ok(ExecResult::Rows(results))
        }

        // ── Aggregation ──
        Statement::Integrate {
            bundle,
            over,
            measures,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;

            if let Some(gb_field) = over {
                // Prefer a real field for SUM/AVG/MIN/MAX; fall back to "*"
                // only when every measure is COUNT(*).
                let agg_field = measures
                    .iter()
                    .map(|m| m.field.as_str())
                    .find(|f| *f != "*")
                    .unwrap_or("*");

                // If any measure is COUNT(*), count every record per
                // group independent of any field's nullness — this is the
                // SQL semantics agg_result.count cannot give us, because
                // agg_result.count tracks records with a non-null
                // agg_field only.
                let needs_count_star = measures
                    .iter()
                    .any(|m| matches!(m.func, AggFunc::Count) && m.field == "*");
                let count_by_group: HashMap<crate::types::Value, usize> =
                    if needs_count_star {
                        let mut m = HashMap::new();
                        if let Some(s) = store.as_heap() {
                            for rec in s.records() {
                                if let Some(k) = rec.get(gb_field) {
                                    *m.entry(k.clone()).or_insert(0usize) += 1;
                                }
                            }
                        }
                        m
                    } else {
                        HashMap::new()
                    };

                let groups = match store.as_heap() {
                    Some(s) => crate::aggregation::group_by(s, gb_field, agg_field),
                    None => HashMap::new(),
                };

                // When COUNT(*) is requested, iterate count_by_group's
                // keys (the superset that includes groups where the
                // chosen agg_field is null for every record). Otherwise
                // use groups' keys for backwards-compatible output.
                let group_keys: Vec<crate::types::Value> = if needs_count_star {
                    count_by_group.keys().cloned().collect()
                } else {
                    groups.keys().cloned().collect()
                };
                let zero_agg = crate::aggregation::AggResult {
                    count: 0,
                    sum: 0.0,
                    sum_sq: 0.0,
                    min: 0.0,
                    max: 0.0,
                };

                let mut rows = Vec::new();
                for key in &group_keys {
                    let agg_result = groups.get(key).unwrap_or(&zero_agg);
                    let mut row = HashMap::new();
                    row.insert(gb_field.clone(), key.clone());
                    for m in measures {
                        let val = match (&m.func, m.field.as_str()) {
                            (AggFunc::Count, "*") => {
                                *count_by_group.get(key).unwrap_or(&0) as f64
                            }
                            (AggFunc::Count, _) => agg_result.count as f64,
                            (AggFunc::Sum, _) => agg_result.sum,
                            (AggFunc::Avg, _) => agg_result.avg(),
                            (AggFunc::Min, _) => agg_result.min,
                            (AggFunc::Max, _) => agg_result.max,
                        };
                        let field_name = m
                            .alias
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field));
                        row.insert(field_name, crate::types::Value::Float(val));
                    }
                    rows.push(row);
                }
                Ok(ExecResult::Rows(rows))
            } else {
                // Global aggregation — no OVER
                let all: Vec<crate::types::Record> = store.records().collect();
                let mut row = HashMap::new();
                for m in measures {
                    // COUNT(*) counts records, not per-field non-null values.
                    if matches!(m.func, AggFunc::Count) && m.field == "*" {
                        let field_name = m.alias.as_ref().cloned().unwrap_or_else(|| {
                            format!("{}_{}", m.func_name(), m.field)
                        });
                        row.insert(
                            field_name,
                            crate::types::Value::Float(all.len() as f64),
                        );
                        continue;
                    }
                    let vals: Vec<f64> = all
                        .iter()
                        .filter_map(|r| r.get(&m.field))
                        .filter_map(|v| match v {
                            crate::types::Value::Integer(n) => Some(*n as f64),
                            crate::types::Value::Float(f) => Some(*f),
                            _ => None,
                        })
                        .collect();
                    let val = match m.func {
                        AggFunc::Count => vals.len() as f64,
                        AggFunc::Sum => vals.iter().sum(),
                        AggFunc::Avg => {
                            if vals.is_empty() {
                                0.0
                            } else {
                                vals.iter().sum::<f64>() / vals.len() as f64
                            }
                        }
                        AggFunc::Min => vals.iter().cloned().fold(f64::INFINITY, f64::min),
                        AggFunc::Max => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    };
                    let field_name = m
                        .alias
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field));
                    row.insert(field_name, crate::types::Value::Float(val));
                }
                Ok(ExecResult::Rows(vec![row]))
            }
        }

        // ── Joins ──
        Statement::Pullback {
            left,
            along,
            right,
            right_field,
            preserve_left: _,
        } => {
            let left_store = engine
                .bundle(left)
                .ok_or_else(|| format!("No bundle: {left}"))?;
            let right_store = engine
                .bundle(right)
                .ok_or_else(|| format!("No bundle: {right}"))?;
            let rf = right_field.as_deref().unwrap_or(along.as_str());
            let joined = match (left_store.as_heap(), right_store.as_heap()) {
                (Some(l), Some(r)) => crate::join::pullback_join(l, r, along, rf),
                _ => Vec::new(),
            };
            let rows: Vec<_> = joined
                .into_iter()
                .map(|(left_rec, right_rec)| {
                    let mut merged = left_rec;
                    if let Some(r) = right_rec {
                        merged.extend(r);
                    }
                    merged
                })
                .collect();
            Ok(ExecResult::Rows(rows))
        }

        // ── SQL compat: SELECT ──
        Statement::Select {
            bundle,
            columns,
            condition,
            group_by,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;

            if let Some(gb_field) = group_by {
                let agg_col = columns.iter().find_map(|c| match c {
                    SelectCol::Agg(func, field) => Some((func, field)),
                    _ => None,
                });
                if let Some((func, field)) = agg_col {
                    let groups = match store.as_heap() {
                        Some(s) => crate::aggregation::group_by(s, gb_field, field),
                        None => HashMap::new(),
                    };
                    let mut rows = Vec::new();
                    for (key, agg_result) in &groups {
                        let mut row = HashMap::new();
                        row.insert(gb_field.clone(), key.clone());
                        let val = match func {
                            AggFunc::Count => agg_result.count as f64,
                            AggFunc::Sum => agg_result.sum,
                            AggFunc::Avg => agg_result.avg(),
                            AggFunc::Min => agg_result.min,
                            AggFunc::Max => agg_result.max,
                        };
                        row.insert(field.clone(), crate::types::Value::Float(val));
                        rows.push(row);
                    }
                    return Ok(ExecResult::Rows(rows));
                }
            }

            match condition {
                Some(Condition::Eq(field, val)) => {
                    let value = literal_to_value(val);
                    let is_base = store.schema().base_fields.iter().any(|f| f.name == *field);
                    if is_base {
                        let mut key = HashMap::new();
                        key.insert(field.clone(), value);
                        match store.point_query(&key) {
                            Some(rec) => Ok(ExecResult::Rows(vec![filter_columns(rec, columns)])),
                            None => Ok(ExecResult::Rows(vec![])),
                        }
                    } else {
                        let results = store.range_query(field, &[value]);
                        let rows: Vec<_> = results
                            .into_iter()
                            .map(|r| filter_columns(r, columns))
                            .collect();
                        Ok(ExecResult::Rows(rows))
                    }
                }
                Some(Condition::Between(field, lo, hi)) => {
                    let lo_val = literal_to_value(lo);
                    let hi_val = literal_to_value(hi);
                    let matching: Vec<crate::types::Value> = store
                        .indexed_values(field)
                        .into_iter()
                        .filter(|v| *v >= lo_val && *v <= hi_val)
                        .collect();
                    let results = store.range_query(field, &matching);
                    let rows: Vec<_> = results
                        .into_iter()
                        .map(|r| filter_columns(r, columns))
                        .collect();
                    Ok(ExecResult::Rows(rows))
                }
                Some(Condition::In(field, vals)) => {
                    let values: Vec<_> = vals.iter().map(literal_to_value).collect();
                    let results = store.range_query(field, &values);
                    let rows: Vec<_> = results
                        .into_iter()
                        .map(|r| filter_columns(r, columns))
                        .collect();
                    Ok(ExecResult::Rows(rows))
                }
                None => {
                    let rows: Vec<_> = store
                        .records()
                        .map(|r| filter_columns(r, columns))
                        .collect();
                    Ok(ExecResult::Rows(rows))
                }
            }
        }

        // ── SQL compat: JOIN ──
        Statement::Join {
            left,
            right,
            on_field,
            columns,
        } => {
            let left_store = engine
                .bundle(left)
                .ok_or_else(|| format!("No bundle: {left}"))?;
            let right_store = engine
                .bundle(right)
                .ok_or_else(|| format!("No bundle: {right}"))?;
            let joined = match (left_store.as_heap(), right_store.as_heap()) {
                (Some(l), Some(r)) => crate::join::pullback_join(l, r, on_field, on_field),
                _ => Vec::new(),
            };
            let rows: Vec<_> = joined
                .into_iter()
                .map(|(left_rec, right_rec)| {
                    let mut merged = left_rec;
                    if let Some(r) = right_rec {
                        merged.extend(r);
                    }
                    filter_columns(merged, columns)
                })
                .collect();
            Ok(ExecResult::Rows(rows))
        }

        // ── Analytics ──
        Statement::Curvature { bundle, .. } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let k = store.as_heap()
                .map(|s| crate::curvature::scalar_curvature(s))
                .unwrap_or_else(|| store.curvature_stats().mean());
            Ok(ExecResult::Scalar(k))
        }

        Statement::Spectral { bundle, .. } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let lambda1 = store.as_heap()
                .map(|s| crate::spectral::spectral_gap(s))
                .unwrap_or(0.0);
            Ok(ExecResult::Scalar(lambda1))
        }

        Statement::Consistency { bundle, repair: _ } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let contradictions = store.as_heap()
                .map(|s| crate::sheaf::consistency_check(s))
                .unwrap_or_default();
            Ok(ExecResult::Rows(contradictions))
        }

        Statement::Complete {
            bundle,
            where_conditions,
            method: _,
            min_confidence,
            with_provenance,
            with_constraint_graph,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let min_conf = min_confidence.unwrap_or(0.30);
            let results = match store.as_heap() {
                Some(s) => crate::sheaf::complete(
                    s,
                    where_conditions,
                    min_conf,
                    *with_provenance,
                    *with_constraint_graph,
                ),
                None => Vec::new(),
            };
            Ok(ExecResult::Rows(results))
        }

        Statement::Propagate {
            bundle,
            assumptions,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let assumption_record = assumptions
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect::<crate::types::Record>();
            let results = match store.as_heap() {
                Some(s) => crate::sheaf::propagate(s, &assumption_record),
                None => Vec::new(),
            };
            Ok(ExecResult::Rows(results))
        }

        Statement::SuggestAdjacency {
            bundle,
            fields,
            sample_size,
            candidates,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let results = store.as_heap()
                .map(|s| crate::sheaf::suggest_adjacency(s, fields, *sample_size, *candidates))
                .unwrap_or_default();
            Ok(ExecResult::Rows(results))
        }

        Statement::Health { bundle } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let k = store.as_heap()
                .map(|s| crate::curvature::scalar_curvature(s))
                .unwrap_or_else(|| store.curvature_stats().mean());
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: crate::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema().base_fields.len(),
                fiber_fields: store.schema().fiber_fields.len(),
            }))
        }

        Statement::Explain { inner: _ } => {
            // Query plan introspection — placeholder
            Ok(ExecResult::Ok)
        }

        Statement::AtlasBegin | Statement::AtlasCommit | Statement::AtlasRollback => {
            // Transaction control is handled at the transport layer
            Ok(ExecResult::Ok)
        }

        // ── v2.1: Access Control (stubs) ──
        Statement::WeaveRole { .. }
        | Statement::UnweaveRole { .. }
        | Statement::ShowRoles
        | Statement::Grant { .. }
        | Statement::Revoke { .. }
        | Statement::CreatePolicy { .. }
        | Statement::DropPolicy { .. }
        | Statement::ShowPolicies { .. }
        | Statement::AuditOn { .. }
        | Statement::AuditOff { .. }
        | Statement::AuditShow { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Constraints (stubs) ──
        Statement::GaugeConstrain { .. }
        | Statement::GaugeUnconstrain { .. }
        | Statement::ShowConstraints { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Maintenance ──
        Statement::Compact { bundle, .. }
        | Statement::Analyze { bundle, .. }
        | Statement::Vacuum { bundle, .. }
        | Statement::RebuildIndex { bundle, .. }
        | Statement::CheckIntegrity { bundle }
        | Statement::Repair { bundle } => {
            let _store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::StorageInfo { bundle } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let k = store.as_heap()
                .map(|s| crate::curvature::scalar_curvature(s))
                .unwrap_or_else(|| store.curvature_stats().mean());
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: 0.0,
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema().base_fields.len(),
                fiber_fields: store.schema().fiber_fields.len(),
            }))
        }

        // ── v2.1: Session (stubs) ──
        Statement::Set { .. }
        | Statement::Reset { .. }
        | Statement::ShowSettings
        | Statement::ShowSession
        | Statement::ShowCurrentRole => Ok(ExecResult::Ok),

        // ── v2.1: Data Movement (stubs) ──
        Statement::Ingest { .. }
        | Statement::Transplant { .. }
        | Statement::GenerateBase { .. }
        | Statement::Fill { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Prepared Statements (stubs) ──
        Statement::Prepare { .. }
        | Statement::Execute { .. }
        | Statement::Deallocate { .. }
        | Statement::ShowPrepared => Ok(ExecResult::Ok),

        // ── v2.1: Backup / Restore (stubs) ──
        Statement::Backup { .. }
        | Statement::Restore { .. }
        | Statement::VerifyBackup { .. }
        | Statement::ShowBackups => Ok(ExecResult::Ok),

        // ── v2.1: Information Schema ──
        Statement::ShowFields { bundle }
        | Statement::ShowIndexes { bundle }
        | Statement::ShowMorphisms { bundle }
        | Statement::ShowTriggers { bundle }
        | Statement::ShowStatistics { bundle }
        | Statement::ShowGeometry { bundle }
        | Statement::ShowComments { bundle } => {
            let _store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            Ok(ExecResult::Ok)
        }

        // ── v2.1: Comments (stub) ──
        Statement::CommentOn { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Recursive (stub) ──
        Statement::Iterate { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Triggers — now wired to engine (Feature #9) ──
        Statement::CreateTrigger {
            event,
            bundle,
            condition: _,
            action,
        } => {
            // Parse event to MutationOp
            let op = if event.contains("INSERT") || event.contains("SECTION") {
                crate::engine::MutationOp::Insert
            } else if event.contains("UPDATE") || event.contains("REDEFINE") {
                crate::engine::MutationOp::Update
            } else if event.contains("DELETE") || event.contains("RETRACT") {
                crate::engine::MutationOp::Delete
            } else {
                crate::engine::MutationOp::Any
            };
            let trigger_name = format!("trigger_{}_{}", bundle, event.replace(' ', "_").to_lowercase());
            let channel = if action.is_empty() { trigger_name.clone() } else { action.clone() };
            let def = crate::engine::TriggerDef {
                name: trigger_name,
                kind: crate::engine::TriggerKind::OnMutation {
                    bundle: bundle.clone(),
                    operation: op,
                    filter: None,
                },
                channel,
            };
            engine.create_trigger(def).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::DropTrigger { name, bundle: _ } => {
            engine.drop_trigger(name).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        // ── Feature #6: Query Cache ──
        Statement::InvalidateCache { bundle } => {
            if let Some(b) = bundle {
                engine.query_cache_mut().invalidate_bundle(b);
            } else {
                engine.query_cache_mut().invalidate_all();
            }
            Ok(ExecResult::Ok)
        }

        Statement::Betti { bundle } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let (b0, b1) = store.betti_numbers();
            Ok(ExecResult::Scalar(b0 as f64 + b1 as f64))
        }
        Statement::Entropy { bundle } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let s = store.entropy();
            Ok(ExecResult::Scalar(s))
        }
        Statement::FreeEnergy { bundle, tau } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let f = store.free_energy(*tau);
            Ok(ExecResult::Scalar(f))
        }
        // ── Cognitive Geometry (Branch VII — Davis 2026-05-29) ──────────────
        Statement::Capacity { bundle, tau } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let k = store.as_heap()
                .map(|s| crate::curvature::scalar_curvature(s))
                .unwrap_or_else(|| store.curvature_stats().mean());
            Ok(ExecResult::Scalar(crate::curvature::capacity(*tau, k)))
        }
        Statement::Horizon { bundle, tau, config } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let heap = store.as_heap();
            let k = heap
                .map(|s| crate::curvature::scalar_curvature(s))
                .unwrap_or_else(|| store.curvature_stats().mean());
            let lambda1 = heap
                .map(|s| crate::spectral::spectral_gap(s))
                .unwrap_or(0.0);
            // The calibrated path needs a BundleStore. If we only have
            // a mmap-overlay view (no heap), fall back to the legacy
            // scalar shim — same behavior as before the calibrated
            // path existed.
            let s_max = if let Some(s) = heap {
                let cfg = config.unwrap_or_default();
                crate::curvature::horizon_with(*tau, k, s, lambda1, &cfg).s_max
            } else {
                crate::curvature::horizon(*tau, k, lambda1)
            };
            Ok(ExecResult::Scalar(s_max))
        }
        Statement::Depth { bundle, config } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let k = store.as_heap()
                .map(|s| crate::curvature::scalar_curvature(s))
                .unwrap_or_else(|| store.curvature_stats().mean());
            let lambda1 = store.as_heap()
                .map(|s| crate::spectral::spectral_gap(s))
                .unwrap_or(0.0);
            let cfg = config.unwrap_or_default();
            let depth = crate::curvature::encoding_depth_with(k, lambda1, &cfg);
            let level: f64 = match depth {
                crate::curvature::EncodingDepth::Tangent     => 1.0,
                crate::curvature::EncodingDepth::Connection  => 2.0,
                crate::curvature::EncodingDepth::Metric      => 3.0,
                crate::curvature::EncodingDepth::Topological => 4.0,
            };
            Ok(ExecResult::Scalar(level))
        }
        Statement::Perceive { bundle, rotation, vector, dim } => {
            // 404-equivalent: bundle must exist (consistent with C/H/D
            // and the HTTP endpoint).
            let _store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            // Default dim = vector length when not supplied.
            let d = dim.unwrap_or(vector.len());
            let res = crate::curvature::perceive(rotation, vector, d)
                .map_err(|e| format!("PERCEIVE: {}", e))?;
            // GQL returns the scalar bias; v_perceived is wire-only
            // (HTTP POST). Scalar bias composes with the rest of GQL
            // (comparisons, EXPLAIN, etc.).
            Ok(ExecResult::Scalar(res.bias))
        }
        Statement::RotateKey { bundle, new_seed_source } => {
            let new_seed = resolve_seed(new_seed_source)?;
            let store = engine
                .heap_bundle_mut(bundle)
                .ok_or_else(|| format!(
                    "ROTATE_KEY requires bundle '{}' to be in heap mode",
                    bundle
                ))?;
            // Sprint G-ext: rotate_key now drives BOTH the gauge key (g)
            // and the base-space hash seed (s) from a single 32-byte
            // master. One call rotates (s, g) → (s', g') atomically.
            let count = store.rotate_key(&new_seed)?;
            Ok(ExecResult::Count(count))
        }
        Statement::ProjectInvariant { bundle, expressions, where_clause } => {
            let bundle_ref = engine
                .bundle(bundle)
                .ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let store = bundle_ref.as_heap().ok_or_else(|| {
                format!(
                    "PROJECT INVARIANT requires bundle '{}' to be in heap mode",
                    bundle
                )
            })?;

            // Sprint H-ext: filtered invariant computation.
            let results: Vec<(String, f64)> = expressions
                .iter()
                .map(|(label, expr)| {
                    let v = match where_clause {
                        Some(conds) => crate::invariant::evaluate_filtered(store, expr, conds),
                        None => crate::invariant::evaluate(store, expr),
                    };
                    (label.clone(), v)
                })
                .collect();
            Ok(ExecResult::Invariants(results))
        }
        Statement::Geodesic { bundle, from_keys, to_keys, max_hops, restrict_bundle } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let from_rec: crate::types::Record = from_keys.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let to_rec: crate::types::Record = to_keys.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let bp_a = store.base_point(&from_rec);
            let bp_b = store.base_point(&to_rec);
            match store.geodesic_path(bp_a, bp_b, *max_hops) {
                Some(path) => {
                    let mut rows: Vec<crate::types::Record> = path.iter().enumerate().map(|(hop, &bp)| {
                        let mut r = crate::types::Record::new();
                        r.insert("hop".to_string(), crate::types::Value::Integer(hop as i64));
                        r.insert("base_point".to_string(), crate::types::Value::Integer(bp as i64));
                        r
                    }).collect();
                    if let Some(rb) = restrict_bundle {
                        let restrict_store = engine.bundle(rb).ok_or_else(|| format!("RESTRICT TO bundle '{}' not found", rb))?;
                        let restrict_bps: std::collections::HashSet<u64> = restrict_store.all_base_points();
                        rows.retain(|r| {
                            r.get("base_point").and_then(|v| v.as_i64()).map(|bp| restrict_bps.contains(&(bp as u64))).unwrap_or(false)
                        });
                    }
                    Ok(ExecResult::Rows(rows))
                }
                None => Ok(ExecResult::Scalar(-1.0)),
            }
        }
        Statement::MetricTensor { bundle } => {
            let store = engine.bundle(bundle).ok_or_else(|| format!("Bundle '{}' not found", bundle))?;
            let info = store.metric_tensor();
            let cond = if info.condition_number.is_finite() { info.condition_number } else { -1.0 };
            Ok(ExecResult::Scalar(cond))
        }
        Statement::HolonomyFiber { .. }
        | Statement::LocalHolonomy { .. }
        | Statement::Transport { .. }
        | Statement::TransportRotation { .. }
        | Statement::GaugeTest { .. }
        | Statement::Divergence { .. }
        | Statement::SpectralFiber { .. }
        | Statement::Ricci { .. }
        | Statement::SectionCoherent { .. }
        | Statement::ShowCharts { .. }
        | Statement::ShowContradictions { .. }
        | Statement::CollapseBranch { .. }
        | Statement::Predict { .. }
        | Statement::CoverGeodesic { .. }
        | Statement::Why { .. }
        | Statement::Implications { .. }
        | Statement::SampleTransport { .. } => {
            Err("This statement must be executed via the HTTP server endpoint".to_string())
        }

        // ── Ask G — Pattern Hunt Phase 2: in-memory registry ──

        #[cfg(feature = "patterns")]
        Statement::DefinePattern {
            name,
            pred,
            or_groups,
            weight,
            using_fields,
            replace,
        } => {
            if !replace && engine.pattern_registry.contains_key(name) {
                return Err(format!(
                    "Pattern '{name}' already exists; use DEFINE OR REPLACE PATTERN to overwrite."
                ));
            }
            engine.pattern_registry.insert(
                name.clone(),
                PatternDef {
                    name: name.clone(),
                    pred: pred.clone(),
                    or_groups: or_groups.clone(),
                    weight: weight.clone(),
                    using_fields: using_fields.clone(),
                },
            );
            Ok(ExecResult::Ok)
        }

        #[cfg(feature = "patterns")]
        Statement::DropPattern { name } => {
            // Idempotent — silently OK when pattern absent. Mirrors
            // DROP TABLE IF EXISTS convention.
            engine.pattern_registry.remove(name);
            Ok(ExecResult::Ok)
        }

        #[cfg(feature = "patterns")]
        Statement::ShowPatterns => {
            // One row per pattern, alphabetized for determinism.
            let mut names: Vec<&String> = engine.pattern_registry.keys().collect();
            names.sort();
            let rows: Vec<crate::types::Record> = names
                .into_iter()
                .map(|n| {
                    let mut record = std::collections::HashMap::new();
                    record.insert("name".to_string(), crate::types::Value::Text(n.clone()));
                    record
                })
                .collect();
            Ok(ExecResult::Rows(rows))
        }

        #[cfg(feature = "patterns")]
        Statement::Hunt {
            pattern,
            bundle,
            excluding,
            extra_on: _,
            extra_where: _,
            rank_by: _,
            top,
            project,
        } => {
            // ── 1-3. Validation (same as Phase 2) ───────────────────────
            let pat = engine
                .pattern_registry
                .get(pattern)
                .ok_or_else(|| format!("Pattern '{pattern}' is not defined."))?
                .clone();
            if engine.bundle(bundle).is_none() {
                return Err(format!("Bundle '{bundle}' does not exist."));
            }

            // Capture the base PK field name FIRST (we'll need it for
            // tie-breaking after the immutable engine borrow ends).
            let base_pk_field: Option<String> = engine
                .heap_bundle(bundle)
                .and_then(|store| store.schema.base_fields.first().map(|f| f.name.clone()));

            if let Some(store) = engine.heap_bundle(bundle) {
                let mut field_names: std::collections::HashSet<&str> =
                    std::collections::HashSet::new();
                for f in &store.schema.base_fields {
                    field_names.insert(f.name.as_str());
                }
                for f in &store.schema.fiber_fields {
                    field_names.insert(f.name.as_str());
                }
                for using in &pat.using_fields {
                    if !field_names.contains(using.as_str()) {
                        return Err(format!(
                            "Pattern '{pattern}' USES field '{using}' \
                             which is missing from bundle '{bundle}'."
                        ));
                    }
                }
            }

            // EXCLUDING IN handling is unified with COVER's via
            // `apply_excluding_in_filter` (defined above). We collect
            // the exclusion PK set up-front so the heap_bundle borrow
            // for `base_pk_field` doesn't conflict with the inner
            // Cover executions in the helper. Filtering happens at
            // step 6b after the main COVER returns.

            // ── 5. Parse WEIGHT expression once per HUNT ────────────────
            let weight_expr: Option<WeightExpr> = match &pat.weight {
                Some(toks) => Some(parse_weight_expr(toks)?),
                None => None,
            };

            // ── 6. Build equivalent COVER + recursively execute ─────────
            //
            // Desugar: HUNT pattern IN bundle → COVER bundle WHERE pred.
            // No PROJECT (we need every field for WEIGHT eval), no RANK BY
            // (we sort by _score ourselves), no FIRST (we TOP-N ourselves).
            let cover = Statement::Cover {
                bundle: bundle.clone(),
                on_conditions: Vec::new(),
                where_conditions: pat.pred.clone(),
                or_groups: pat.or_groups.clone(),
                distinct_field: None,
                project: None,
                rank_by: None,
                first: None,
                skip: None,
                all: true,
                excluding: Vec::new(),
            };
            let mut rows: Vec<crate::types::Record> = match execute(engine, &cover)? {
                ExecResult::Rows(rs) => rs,
                other => {
                    return Err(format!(
                        "HUNT inner COVER returned non-Rows result: {other:?}"
                    ));
                }
            };

            // ── 6b. Apply EXCLUDING IN anti-join via shared helper ──────
            apply_excluding_in_filter(engine, excluding, &base_pk_field, &mut rows)?;

            // ── 7. Evaluate WEIGHT → augment rows with `_score` ─────────
            for row in rows.iter_mut() {
                let score = match &weight_expr {
                    Some(expr) => eval_weight(expr, row),
                    None => 0.0,
                };
                row.insert(
                    "_score".to_string(),
                    crate::types::Value::Float(score),
                );
            }

            // ── 8. Sort: _score DESC, tie-break by base PK ASC ──────────
            let pk_field = base_pk_field.unwrap_or_default();
            rows.sort_by(|a, b| {
                let sa = a.get("_score").and_then(value_to_f64).unwrap_or(0.0);
                let sb = b.get("_score").and_then(value_to_f64).unwrap_or(0.0);
                // DESC by score (NaN sorts to bottom per spec §10).
                let primary = sb
                    .partial_cmp(&sa)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if primary != std::cmp::Ordering::Equal {
                    return primary;
                }
                // ASC by base PK on ties.
                if pk_field.is_empty() {
                    return std::cmp::Ordering::Equal;
                }
                let pa = a.get(&pk_field).and_then(value_to_f64).unwrap_or(0.0);
                let pb = b.get(&pk_field).and_then(value_to_f64).unwrap_or(0.0);
                pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
            });

            // ── 9. TOP n truncation ─────────────────────────────────────
            if let Some(n) = top {
                rows.truncate(*n);
            }

            // ── 10. Apply user's PROJECT (if any) ───────────────────────
            // PROJECT (a, b, _score) → return only those fields.
            // Missing PROJECT → return all fields (including _score).
            if let Some(fields) = project {
                let filtered: Vec<crate::types::Record> = rows
                    .into_iter()
                    .map(|row| {
                        let mut new_row = std::collections::HashMap::new();
                        for field in fields {
                            if let Some(v) = row.get(field) {
                                new_row.insert(field.clone(), v.clone());
                            }
                        }
                        new_row
                    })
                    .collect();
                Ok(ExecResult::Rows(filtered))
            } else {
                Ok(ExecResult::Rows(rows))
            }
        }

        // ── LATTICE executor arms (gated on `lattice` feature) ──
        //
        // Explicit `LATTICE name VERTICES … EDGES … FACES … TOPOLOGY …;`
        // — the parser packs the canonical GQL re-emit form into
        // `gql`; the executor round-trips it through
        // `Lattice::from_gql` and registers under the lattice's
        // name.
        #[cfg(feature = "lattice")]
        Statement::Lattice { name: _name, gql } => {
            let lat = crate::lattice::Lattice::from_gql(gql)
                .map_err(|e| format!("LATTICE re-parse failed: {e}"))?;
            crate::lattice::registry::register(lat);
            Ok(ExecResult::Ok)
        }

        // Shorthand: `LATTICE name FROM TRUNCATED_ICOSAHEDRON
        // [TOPOLOGY "S"];`. Only `TRUNCATED_ICOSAHEDRON` is wired
        // at launch — see `HALCYON_PART_I_GATES.md` Part I scope.
        //
        // TDD-HAL-V.4: when the `gauge` feature is on, the canonical
        // lattice declaration routes through `declare_lattice_durable`
        // so a downstream `GAUGE_FIELD … PERSIST` (and the SNAPSHOT
        // chain it gates) survives engine close + reopen. The V.4
        // smoke gate's failure mode without this fix is:
        //   GAUGE_FIELD PERSIST writes its WAL entry referencing
        //   `buckyball` ⇒ engine close ⇒ engine reopen ⇒ WAL replay
        //   Pass 2 fails with "GaugeFieldDeclare references unknown
        //   lattice 'buckyball'" because Pass 1 (lattice declares)
        //   walked an empty set.
        // With `gauge` off the in-memory `register` path stays —
        // the no-default-features build cannot reach the durable arm
        // because `declare_lattice_durable` is itself gated on
        // `gauge`. Optionality contract: 852/0 byte-identical stays
        // intact.
        #[cfg(feature = "lattice")]
        Statement::LatticeFromCanonical {
            name,
            canonical,
            topology,
        } => {
            let mut lat = {
                let canonical_upper = canonical.to_ascii_uppercase();
                let constructor = crate::lattice::registry::get_constructor(
                    &canonical_upper,
                )
                .ok_or_else(|| {
                    format!(
                        "Unknown canonical lattice constructor: '{canonical}'. \
                         Phase 1 ships TRUNCATED_ICOSAHEDRON and CUBED_SPHERE."
                    )
                })?;
                let args = crate::lattice::registry::ConstructorArgs::default();
                let lwm = constructor(&args).map_err(|e| {
                    format!(
                        "lattice constructor for '{canonical}' failed: {e:?}"
                    )
                })?;
                lwm.lattice().clone()
            };
            // Rename to the user-supplied name, override topology
            // hint if one was supplied. Buckyball's default
            // topology is `"S2"` — leave that intact unless the
            // statement overrides it.
            lat.name = name.clone();
            if let Some(t) = topology {
                lat.topology = Some(t.clone());
            }
            #[cfg(feature = "gauge")]
            {
                // Durable path — WAL-log the declaration so the
                // snapshot smoke gate (V.4) round-trips through
                // close + reopen. `declare_lattice_durable` writes
                // the lattice's canonical GQL form to the WAL and
                // installs it in the in-process registry.
                engine
                    .declare_lattice_durable(lat)
                    .map_err(|e| format!("lattice: durable declaration failed: {e}"))?;
            }
            #[cfg(not(feature = "gauge"))]
            {
                crate::lattice::registry::register(lat);
            }
            Ok(ExecResult::Ok)
        }

        // `SHOW LATTICE name;` — emit the registered lattice's
        // canonical GQL form on the single returned row's `gql`
        // column. The integration test in `src/lattice/registry.rs`
        // round-trips this through `parse_gql` and verifies
        // structural equality.
        #[cfg(feature = "lattice")]
        Statement::ShowLattice { name } => {
            let lat = crate::lattice::registry::get(name)
                .ok_or_else(|| format!("Lattice '{name}' is not registered"))?;
            let gql = lat.to_gql();
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert("name".to_string(), crate::types::Value::Text(lat.name));
            record.insert("gql".to_string(), crate::types::Value::Text(gql));
            record.insert(
                "n_vertices".to_string(),
                crate::types::Value::Integer(lat.n_vertices as i64),
            );
            record.insert(
                "n_edges".to_string(),
                crate::types::Value::Integer(lat.edges.len() as i64),
            );
            record.insert(
                "n_faces".to_string(),
                crate::types::Value::Integer(lat.faces.len() as i64),
            );
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── GAUGE_FIELD executor arms (gated on `gauge` feature) ──
        //
        // `GAUGE_FIELD name ON LATTICE lat GROUP G INIT … [PERSIST];`
        // closes the executor half of TDD-HAL-II.5. Bee's locked
        // decisions wired through:
        //   1. (CSPRNG / bit-identity) — `SU2GaugeField::new` calls
        //      `DenseLinkBuffer::new_haar` which runs the Marsaglia
        //      sampler through GIGI's own SmallRng (xorshift64*); same
        //      seed → byte-identical buffer.
        //   3. (PERSIST routing) — explicit `PERSIST` keyword routes to
        //      `engine.declare_gauge_field_durable`; default routes to
        //      the in-memory `gauge::registry::register`.
        //   5. (typed errors) — every failure mode surfaces as a
        //      `GaugeFieldError` Display string; Halcyon's G2.D regex
        //      anchor `SU\(2\)` matches against
        //      `UnsupportedGroup(group).to_string()`.
        //   6. (group erasure) — the parser accepts every Group
        //      variant; non-SU(2) variants fail here with
        //      `UnsupportedGroup`.
        #[cfg(feature = "gauge")]
        Statement::GaugeField {
            name,
            lattice,
            group,
            init,
            seed,
            persist,
        } => {
            use crate::gauge::{GaugeFieldError, GaugeFieldInit, Group, SU2GaugeField};

            // 1. Resolve the bound lattice. The `LatticeNotDeclared`
            //    Display contains the literal "not declared" so the
            //    Halcyon G2.B regex anchor `not declared` matches.
            let lat = crate::lattice::registry::get(lattice).ok_or_else(|| {
                GaugeFieldError::LatticeNotDeclared(lattice.clone()).to_string()
            })?;

            // 2. Group-erasure dispatch — only SU(2) proceeds to
            //    construction at launch. Bee's locked decision 6.
            if !matches!(group, Group::SU2) {
                return Err(GaugeFieldError::UnsupportedGroup(*group).to_string());
            }

            // 3. Resolve `INIT FROM other` if requested — clone the
            //    source field's buffer + metadata. Otherwise hand off
            //    to `SU2GaugeField::new` directly.
            let field = match init {
                GaugeFieldInit::FromField(src) => {
                    let src_handle =
                        crate::gauge::registry::get(src).ok_or_else(|| {
                            GaugeFieldError::FieldNotDeclared(src.clone()).to_string()
                        })?;
                    // Source must live on the same lattice (the
                    // walker reads through the bound lattice's edge
                    // ids; a foreign-lattice clone would silently
                    // mis-index).
                    if src_handle.lattice_name() != lat.name {
                        return Err(format!(
                            "gauge: INIT FROM source '{}' lives on lattice '{}', \
                             not '{}'",
                            src,
                            src_handle.lattice_name(),
                            lat.name
                        ));
                    }
                    let src_buf = src_handle.as_dense_buffer().clone();
                    SU2GaugeField {
                        name: name.clone(),
                        lattice_name: lat.name.clone(),
                        buffer: src_buf,
                        init_kind: GaugeFieldInit::FromField(src.clone()),
                        init_seed: None,
                    }
                }
                _ => SU2GaugeField::new(name.clone(), &lat, init.clone(), *seed)
                    .map_err(|e| e.to_string())?,
            };

            // Declaration must populate BOTH the dyn read map AND the
            // SU(2)-mut sibling map so downstream mutators (GIBBS_SAMPLE
            // / SYMPLECTIC_FLOW / SNAPSHOT) can find the field via
            // `get_su2_mut`. `register_su2` already covers both maps
            // (see src/gauge/registry.rs:160-176), so the non-PERSIST
            // branch is a single call. The PERSIST branch still needs
            // the Arc<dyn> handle for `declare_gauge_field_durable` and
            // a separate `register_su2(field_snapshot)` afterwards for
            // the SU(2)-mut sibling.
            if *persist {
                let field_snapshot = field.clone();
                let handle: std::sync::Arc<dyn crate::gauge::registry::GaugeFieldHandle> =
                    std::sync::Arc::new(field);
                engine
                    .declare_gauge_field_durable(handle)
                    .map_err(|e| format!("gauge: PERSIST failed: {e}"))?;
                crate::gauge::registry::register_su2(field_snapshot);
            } else {
                crate::gauge::registry::register_su2(field);
            }
            Ok(ExecResult::Ok)
        }

        // `SHOW GAUGE_FIELD name [BUFFER];` — emit the registered
        // gauge field's metadata (and optionally the full
        // `(n_edges, repr_dim)` buffer) as a single-row result. The
        // BUFFER form materializes the JSON envelope's `data` field
        // (Bee's locked decision 4) as a row-major Vec<Vec<f64>>.
        #[cfg(feature = "gauge")]
        Statement::ShowGaugeField { name, with_buffer } => {
            let handle = crate::gauge::registry::get(name).ok_or_else(|| {
                crate::gauge::GaugeFieldError::FieldNotDeclared(name.clone()).to_string()
            })?;
            let buf = handle.as_dense_buffer();
            let (kind, init_seed) = handle.init_metadata();
            let init_kind = match &kind {
                crate::gauge::GaugeFieldInit::Identity => "IDENTITY",
                crate::gauge::GaugeFieldInit::HaarRandom => "HAAR_RANDOM",
                crate::gauge::GaugeFieldInit::FromField(_) => "FROM_FIELD",
            };
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "name".into(),
                crate::types::Value::Text(handle.name().to_string()),
            );
            record.insert(
                "lattice".into(),
                crate::types::Value::Text(handle.lattice_name().to_string()),
            );
            record.insert(
                "group".into(),
                crate::types::Value::Text(handle.group().label().to_string()),
            );
            record.insert(
                "repr_dim".into(),
                crate::types::Value::Integer(buf.repr_dim as i64),
            );
            record.insert(
                "n_edges".into(),
                crate::types::Value::Integer(buf.n_edges as i64),
            );
            record.insert(
                "init_kind".into(),
                crate::types::Value::Text(init_kind.to_string()),
            );
            record.insert(
                "init_seed".into(),
                match init_seed {
                    Some(s) => crate::types::Value::Integer(s as i64),
                    None => crate::types::Value::Null,
                },
            );
            // FROM_FIELD source name (round-trips through SHOW for
            // declarations cloned from another field).
            if let crate::gauge::GaugeFieldInit::FromField(src) = &kind {
                record.insert(
                    "init_from".into(),
                    crate::types::Value::Text(src.clone()),
                );
            }
            if *with_buffer {
                // Row-major (n_edges, repr_dim) packed into a
                // Vec<Value::Vector>. The HTTP surface (II.6) lifts
                // this into the JSON envelope's `data` field.
                let mut rows: Vec<crate::types::Value> =
                    Vec::with_capacity(buf.n_edges);
                let d = buf.repr_dim;
                for e in 0..buf.n_edges {
                    let slice = &buf.data[e * d..(e + 1) * d];
                    rows.push(crate::types::Value::Vector(slice.to_vec()));
                }
                record.insert(
                    "data".into(),
                    crate::types::Value::Vector(
                        // Flatten row-major into a single Vector so
                        // the existing `Value` surface stays
                        // unchanged. `repr_dim` + `n_edges` recover
                        // shape.
                        buf.data.clone(),
                    ),
                );
                // Also expose the per-row split for callers that
                // want it without re-chunking. Stored under
                // `data_rows` as a Vec-of-Vectors via repeated
                // Vector entries is not representable in the flat
                // Value enum, so we expose `data_flat_len` as the
                // sanity-check companion.
                let _ = rows; // keep the row split logic alive for
                              // future structural columns; today the
                              // flat representation is the single
                              // wire format.
                record.insert(
                    "data_flat_len".into(),
                    crate::types::Value::Integer((buf.n_edges * d) as i64),
                );
            }
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-III.6 — SELECT PLAQUETTE OF U projection ────────
        // Three reductions (per-face / mean / sum) lower to the III.1
        // primitives. The per-face form returns a Vec<f64> of length
        // F packed into a Value::Vector; the mean / sum reductions
        // return a Value::Float scalar. Both wire shapes carry the
        // `reduction` label so JSON consumers can disambiguate
        // without re-reading the GQL source.
        #[cfg(feature = "gauge")]
        Statement::SelectPlaquette { field, reduction } => {
            use crate::gauge::PlaquetteReduction;
            let handle = crate::gauge::registry::get(field).ok_or_else(|| {
                crate::gauge::GaugeFieldError::FieldNotDeclared(field.clone()).to_string()
            })?;
            let lat = crate::lattice::registry::get(handle.lattice_name()).ok_or_else(
                || {
                    crate::gauge::GaugeFieldError::LatticeNotDeclared(
                        handle.lattice_name().to_string(),
                    )
                    .to_string()
                },
            )?;
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "field".into(),
                crate::types::Value::Text(field.clone()),
            );
            record.insert(
                "reduction".into(),
                crate::types::Value::Text(reduction.label().to_string()),
            );
            match reduction {
                PlaquetteReduction::PerFace => {
                    let v = crate::gauge::plaquette_per_face(handle.as_ref(), &lat)
                        .map_err(|e| e.to_string())?;
                    record.insert(
                        "n_faces".into(),
                        crate::types::Value::Integer(v.len() as i64),
                    );
                    record.insert("per_face".into(), crate::types::Value::Vector(v));
                }
                PlaquetteReduction::Mean => {
                    let m = crate::gauge::plaquette_mean(handle.as_ref(), &lat)
                        .map_err(|e| e.to_string())?;
                    record.insert("value".into(), crate::types::Value::Float(m));
                }
                PlaquetteReduction::Sum => {
                    let s = crate::gauge::plaquette_sum(handle.as_ref(), &lat)
                        .map_err(|e| e.to_string())?;
                    record.insert("value".into(), crate::types::Value::Float(s));
                }
            }
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-III.6 — SELECT Q_SURROGATE OF U projection ──────
        // Scalar f64 (locked decision D6 — mirrors the Halcyon mock
        // byte-for-byte at the JSON level).
        #[cfg(feature = "gauge")]
        Statement::SelectQSurrogate { field } => {
            let handle = crate::gauge::registry::get(field).ok_or_else(|| {
                crate::gauge::GaugeFieldError::FieldNotDeclared(field.clone()).to_string()
            })?;
            let lat = crate::lattice::registry::get(handle.lattice_name()).ok_or_else(
                || {
                    crate::gauge::GaugeFieldError::LatticeNotDeclared(
                        handle.lattice_name().to_string(),
                    )
                    .to_string()
                },
            )?;
            let q = crate::gauge::q_surrogate(handle.as_ref(), &lat)
                .map_err(|e| e.to_string())?;
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "field".into(),
                crate::types::Value::Text(field.clone()),
            );
            record.insert("value".into(), crate::types::Value::Float(q));
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-III.6 — GIBBS_SAMPLE executor arm ───────────────
        // Group-erasure dispatch (locked decision D4): handle.group()
        // matches `Group::SU2` and routes into `gauge::gibbs_sample`.
        // Future SU(3) Cabibbo-Marinari heatbath ships
        // `gauge::gibbs_sample_su3` and adds a new arm here without
        // touching the grammar. The response is a single-row Rows
        // envelope carrying `field`, `seed`, `beta`,
        // `n_sweeps_completed`, and one column per measured
        // observable (`mean_plaquette`, `q_surrogate`, …) as a
        // `Value::Vector` of measurement-chain f64s.
        #[cfg(feature = "gauge")]
        Statement::GibbsSample {
            field,
            beta,
            n_sweeps,
            measure_every,
            measure,
            seed,
        } => {
            // 1. Resolve the field through the dyn surface so the
            //    typed `FieldNotDeclared` Display anchors before we
            //    touch the SU(2)-mut handle.
            let handle = crate::gauge::registry::get(field).ok_or_else(|| {
                crate::gauge::GaugeFieldError::FieldNotDeclared(field.clone()).to_string()
            })?;
            // 2. Group-erasure dispatch — only SU(2) proceeds to the
            //    heatbath kernel.
            if !matches!(handle.group(), crate::gauge::Group::SU2) {
                return Err(crate::gauge::GaugeFieldError::UnsupportedGroup(
                    handle.group(),
                )
                .to_string());
            }
            // 3. Drive the III.5 primitive. SeedRequired surfaces as
            //    the typed error string the upstream regex anchors
            //    expect ("SEED" substring).
            let resp = crate::gauge::gibbs_sample(
                field,
                *beta,
                *n_sweeps,
                *measure_every,
                measure.clone(),
                *seed,
            )
            .map_err(|e| e.to_string())?;
            // 4. Lower the response into a Rows envelope. One row
            //    per call; per-observable measurement chains live
            //    under the observable's `label()` column.
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "field".into(),
                crate::types::Value::Text(resp.field.clone()),
            );
            record.insert(
                "seed".into(),
                crate::types::Value::Integer(resp.diagnostics.seed as i64),
            );
            record.insert(
                "beta".into(),
                crate::types::Value::Float(resp.diagnostics.beta),
            );
            record.insert(
                "n_sweeps_completed".into(),
                crate::types::Value::Integer(
                    resp.diagnostics.n_sweeps_completed as i64,
                ),
            );
            for (obs, chain) in &resp.measurement_history {
                record.insert(
                    obs.label().to_string(),
                    crate::types::Value::Vector(chain.clone()),
                );
            }
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-IV.7 — E_FIELD declaration ──────────────────────
        //
        // Group-erasure dispatch (locked decision IV-B): the parser is
        // group-agnostic; we resolve the source U through the dyn read
        // registry, match `handle.group()`, and dispatch into the
        // SU(2) sibling kernel for now. Future U(1)/SU(3) E fields
        // land as new executor arms with their own EField constructors.
        //
        // The new SU2EField is wrapped in `Arc<Mutex<_>>` and registered
        // via `register_su2_e`; subsequent SYMPLECTIC_FLOW / SHOW E_FIELD
        // / SELECT H_TOTAL invocations read through that handle.
        #[cfg(feature = "gauge")]
        Statement::EField {
            name,
            source_gauge_field,
            init,
            seed,
        } => {
            use crate::gauge::{GaugeFieldError, Group, SU2EField};
            // 1. Resolve the source U through the dyn surface so the
            //    typed `FieldNotDeclared` Display anchors before we
            //    touch the E constructor.
            let u_handle = crate::gauge::registry::get(source_gauge_field).ok_or_else(
                || {
                    GaugeFieldError::FieldNotDeclared(source_gauge_field.clone())
                        .to_string()
                },
            )?;
            // 2. Group-erasure dispatch — only SU(2) proceeds.
            if !matches!(u_handle.group(), Group::SU2) {
                return Err(GaugeFieldError::UnsupportedGroup(u_handle.group()).to_string());
            }
            // 3. Construct the SU(2) E field. The constructor performs
            //    the FromField lookup itself (registry handle resolution)
            //    so we hand off `init` directly. SeedRequired /
            //    EFieldNotDeclared / EFieldSourceMismatch surface as
            //    typed errors.
            let e_field = SU2EField::new(
                name.clone(),
                u_handle.as_ref(),
                init.clone(),
                *seed,
            )
            .map_err(|e| e.to_string())?;
            // 4. Park it in the SU(2) E-field sibling registry.
            crate::gauge::registry::register_su2_e(std::sync::Arc::new(
                std::sync::Mutex::new(e_field),
            ));
            Ok(ExecResult::Ok)
        }

        // ── TDD-HAL-IV.7 — SHOW E_FIELD ───────────────────────────────
        //
        // Without BUFFER: metadata-only row (name / source_gauge_field /
        // source_lattice / group / n_edges / init_kind / init_seed).
        // With BUFFER: same metadata + the full `(n_edges, 4)` Lie
        // buffer in the `data` column (q0=0 invariant on every row).
        #[cfg(feature = "gauge")]
        Statement::ShowEField { name, with_buffer } => {
            use crate::gauge::EFieldInit;
            let handle = crate::gauge::registry::get_su2_e(name).ok_or_else(|| {
                crate::gauge::GaugeFieldError::EFieldNotDeclared(name.clone()).to_string()
            })?;
            let buf = handle.as_dense_buffer();
            let (init, init_seed) = handle.init_metadata();
            let init_kind = match &init {
                EFieldInit::Zero => "ZERO",
                EFieldInit::MaxwellBoltzmann { .. } => "MAXWELL_BOLTZMANN",
                EFieldInit::FromField(_) => "FROM_FIELD",
            };
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "name".into(),
                crate::types::Value::Text(handle.name().to_string()),
            );
            record.insert(
                "source_gauge_field".into(),
                crate::types::Value::Text(handle.source_gauge_field().to_string()),
            );
            record.insert(
                "source_lattice".into(),
                crate::types::Value::Text(handle.source_lattice().to_string()),
            );
            record.insert(
                "group".into(),
                crate::types::Value::Text(handle.group().label().to_string()),
            );
            record.insert(
                "n_edges".into(),
                crate::types::Value::Integer(buf.n_edges as i64),
            );
            record.insert(
                "init_kind".into(),
                crate::types::Value::Text(init_kind.to_string()),
            );
            record.insert(
                "init_seed".into(),
                match init_seed {
                    Some(s) => crate::types::Value::Integer(s as i64),
                    None => crate::types::Value::Null,
                },
            );
            // Round-trip the MAXWELL_BOLTZMANN β and FROM_FIELD source
            // name through SHOW for declarations that need them.
            match &init {
                EFieldInit::MaxwellBoltzmann { beta } => {
                    record.insert(
                        "init_beta".into(),
                        crate::types::Value::Float(*beta),
                    );
                }
                EFieldInit::FromField(src) => {
                    record.insert(
                        "init_from".into(),
                        crate::types::Value::Text(src.clone()),
                    );
                }
                EFieldInit::Zero => {}
            }
            if *with_buffer {
                record.insert(
                    "data".into(),
                    crate::types::Value::Vector(buf.data.clone()),
                );
                record.insert(
                    "data_flat_len".into(),
                    crate::types::Value::Integer((buf.n_edges * buf.repr_dim) as i64),
                );
            }
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-IV.7 — SELECT H_TOTAL OF (U, E) ──────────────────
        //
        // E-aware Hamiltonian read. Reverses Part III's
        // PartIvObservableNotReady stub at the SELECT-projection layer
        // (locked decision IV-J). Returns a scalar f64. We DO NOT
        // mutate state — both handles are read-only.
        #[cfg(feature = "gauge")]
        Statement::SelectHTotal {
            gauge_field,
            e_field,
        } => {
            use crate::gauge::{GaugeFieldError, Group};
            // Resolve U handle through dyn surface.
            let u_handle = crate::gauge::registry::get(gauge_field).ok_or_else(|| {
                GaugeFieldError::FieldNotDeclared(gauge_field.clone()).to_string()
            })?;
            if !matches!(u_handle.group(), Group::SU2) {
                return Err(GaugeFieldError::UnsupportedGroup(u_handle.group()).to_string());
            }
            // Resolve E handle through the SU(2) E sibling registry.
            let e_arc = crate::gauge::registry::get_su2_e_mut(e_field).ok_or_else(|| {
                GaugeFieldError::EFieldNotDeclared(e_field.clone()).to_string()
            })?;
            // SELECT H_TOTAL needs a concrete SU2GaugeField, not a dyn
            // handle (the Hamiltonian formula reads U through the
            // concrete buffer). We resolve through the SU(2)-mut map
            // for symmetry with SYMPLECTIC_FLOW; this is a read-only
            // path (no buffer mutation), but the mut handle is the
            // only one that exposes the concrete SU2GaugeField.
            let u_arc = crate::gauge::registry::get_su2_mut(gauge_field).ok_or_else(
                || GaugeFieldError::FieldNotDeclared(gauge_field.clone()).to_string(),
            )?;
            let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
            let e_guard = e_arc.lock().expect("e field mutex poisoned");
            // Sanity: the two handles must share a lattice (locked
            // decision IV-B + IV-J — joint observables on (U, E) only
            // make sense when both bind to the same lattice).
            if e_guard.source_lattice != u_guard.lattice_name {
                return Err(GaugeFieldError::EFieldSourceMismatch {
                    e_lattice: e_guard.source_lattice.clone(),
                    u_lattice: u_guard.lattice_name.clone(),
                }
                .to_string());
            }
            let lat = crate::lattice::registry::get(&u_guard.lattice_name)
                .ok_or_else(|| {
                    GaugeFieldError::LatticeNotDeclared(u_guard.lattice_name.clone())
                        .to_string()
                })?;
            // Drive the symplectic_flow module's H_total helper through
            // the public re-export `select_h_total`. We compute the
            // Hamiltonian inline here using the same formulae the
            // symplectic_flow's `observe` arm uses (the inline call
            // keeps the module surface narrow — no need to publish a
            // module-level `compute_hamiltonian` just for this verb).
            //
            // The formula (Halcyon Kogut-Susskind, g²=4/β):
            //   H = K + V
            //   K = g² · Σ_e |E_vec|²
            //   V = (F/g²) · (1 - ⟨P⟩)
            let beta_default: f64 = {
                // We need a β to evaluate H_total. The Halcyon
                // convention pins β at the E field's init metadata
                // when MaxwellBoltzmann; for Zero/FromField we use a
                // sentinel 1.0 so the read still completes. Halcyon's
                // production flow ALWAYS measures H_total inside
                // SYMPLECTIC_FLOW where β is the call's β; the
                // SELECT-projection path is the diagnostic surface
                // ("show me the current H given the field's natural
                // β").
                match &e_guard.init_kind {
                    crate::gauge::EFieldInit::MaxwellBoltzmann { beta } => *beta,
                    _ => 1.0_f64,
                }
            };
            let g2 = 4.0_f64 / beta_default;
            let mut kin = 0.0_f64;
            for edge in 0..e_guard.buffer.n_edges {
                let row = e_guard.read_element_q(edge);
                kin += row[1] * row[1] + row[2] * row[2] + row[3] * row[3];
            }
            let kin = g2 * kin;
            let p_mean = crate::gauge::plaquette_mean(&*u_guard, &lat)
                .map_err(|e| e.to_string())?;
            let s_w = (lat.n_faces() as f64) * (1.0_f64 - p_mean) / g2;
            let h_total = kin + s_w;
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "gauge_field".into(),
                crate::types::Value::Text(gauge_field.clone()),
            );
            record.insert(
                "e_field".into(),
                crate::types::Value::Text(e_field.clone()),
            );
            record.insert("value".into(), crate::types::Value::Float(h_total));
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-IV.7 — SELECT GAUSS_RESIDUAL_MAX OF (U, E) ────────
        //
        // Joint observable on (U, E). Default reduction is Covariant
        // (Halcyon production-canonical); Flat reduction is reserved
        // for future debug paths.
        #[cfg(feature = "gauge")]
        Statement::SelectGaussResidualMax {
            gauge_field,
            e_field,
            reduction,
        } => {
            use crate::gauge::{GaugeFieldError, GaussReduction, Group};
            let u_handle = crate::gauge::registry::get(gauge_field).ok_or_else(|| {
                GaugeFieldError::FieldNotDeclared(gauge_field.clone()).to_string()
            })?;
            if !matches!(u_handle.group(), Group::SU2) {
                return Err(GaugeFieldError::UnsupportedGroup(u_handle.group()).to_string());
            }
            let e_arc = crate::gauge::registry::get_su2_e_mut(e_field).ok_or_else(|| {
                GaugeFieldError::EFieldNotDeclared(e_field.clone()).to_string()
            })?;
            let u_arc = crate::gauge::registry::get_su2_mut(gauge_field).ok_or_else(
                || GaugeFieldError::FieldNotDeclared(gauge_field.clone()).to_string(),
            )?;
            let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
            let e_guard = e_arc.lock().expect("e field mutex poisoned");
            if e_guard.source_lattice != u_guard.lattice_name {
                return Err(GaugeFieldError::EFieldSourceMismatch {
                    e_lattice: e_guard.source_lattice.clone(),
                    u_lattice: u_guard.lattice_name.clone(),
                }
                .to_string());
            }
            let lat = crate::lattice::registry::get(&u_guard.lattice_name)
                .ok_or_else(|| {
                    GaugeFieldError::LatticeNotDeclared(u_guard.lattice_name.clone())
                        .to_string()
                })?;
            let vinc = crate::gauge::build_vertex_edge_incidence(&lat);
            let residuals = match reduction {
                GaussReduction::Covariant => {
                    crate::gauge::compute_gauss_residual_covariant(
                        &*u_guard, &*e_guard, &lat, &vinc,
                    )
                    .map_err(|e| e.to_string())?
                }
                GaussReduction::Flat => crate::gauge::compute_gauss_residual_flat(
                    &*e_guard, &lat, &vinc,
                )
                .map_err(|e| e.to_string())?,
            };
            let max = crate::gauge::max_inf_norm(&residuals);
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "gauge_field".into(),
                crate::types::Value::Text(gauge_field.clone()),
            );
            record.insert(
                "e_field".into(),
                crate::types::Value::Text(e_field.clone()),
            );
            record.insert(
                "reduction".into(),
                crate::types::Value::Text(reduction.label().to_string()),
            );
            record.insert("value".into(), crate::types::Value::Float(max));
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-IV.7 — SYMPLECTIC_FLOW executor arm ──────────────
        //
        // Group-erasure dispatch (locked decision IV-B): handle.group()
        // matches `Group::SU2` and routes into
        // `gauge::symplectic_flow`. Future SU(3) Cabibbo-Marinari flow
        // ships `gauge::symplectic_flow_su3` and adds a new arm here
        // without touching the grammar.
        #[cfg(feature = "gauge")]
        Statement::SymplecticFlow {
            field,
            e_field,
            beta,
            dt,
            n_steps,
            project_gauss,
            measure_every,
            measure,
            seed,
        } => {
            use crate::gauge::{
                GaugeFieldError, Group, SymplecticFlowConfig,
            };
            // Resolve the U handle through the dyn surface for the
            // group-erasure dispatch. The SU(2)-mut handle is acquired
            // inside `symplectic_flow` via `get_su2_mut`.
            let u_handle = crate::gauge::registry::get(field).ok_or_else(|| {
                GaugeFieldError::FieldNotDeclared(field.clone()).to_string()
            })?;
            if !matches!(u_handle.group(), Group::SU2) {
                return Err(GaugeFieldError::UnsupportedGroup(u_handle.group()).to_string());
            }
            let config = SymplecticFlowConfig {
                beta: *beta,
                dt: *dt,
                n_steps: *n_steps,
                project_gauss: *project_gauss,
                measure_every: *measure_every,
                measure: measure.clone(),
            };
            let resp = crate::gauge::symplectic_flow(field, e_field, config, *seed)
                .map_err(|e| e.to_string())?;
            // Lower the response into a Rows envelope.
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "run_id".into(),
                crate::types::Value::Text(resp.run_id.clone()),
            );
            record.insert(
                "field".into(),
                crate::types::Value::Text(resp.field.clone()),
            );
            record.insert(
                "e_field".into(),
                crate::types::Value::Text(resp.e_field.clone()),
            );
            record.insert(
                "seed".into(),
                match resp.diagnostics.seed {
                    Some(s) => crate::types::Value::Integer(s as i64),
                    None => crate::types::Value::Null,
                },
            );
            record.insert(
                "beta".into(),
                crate::types::Value::Float(resp.diagnostics.beta),
            );
            record.insert(
                "dt".into(),
                crate::types::Value::Float(resp.diagnostics.dt),
            );
            record.insert(
                "n_steps_completed".into(),
                crate::types::Value::Integer(
                    resp.diagnostics.n_steps_completed as i64,
                ),
            );
            record.insert(
                "cg_iterations_per_step_p99".into(),
                crate::types::Value::Float(
                    resp.diagnostics.cg_iterations_per_step_p99,
                ),
            );
            record.insert(
                "max_energy_drift_rel".into(),
                crate::types::Value::Float(resp.diagnostics.max_energy_drift_rel),
            );
            record.insert(
                "gauss_residual_max".into(),
                crate::types::Value::Float(resp.diagnostics.gauss_residual_max),
            );
            for (obs, chain) in &resp.measurement_history {
                record.insert(
                    obs.label().to_string(),
                    crate::types::Value::Vector(chain.clone()),
                );
            }
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-VI.2 — LOOP declaration executor arm ────────────
        //
        // Resolves a `LOOP name ON lattice (FACE n | EDGES (v0,…))`
        // into a cached edge list and registers it in the
        // `gauge::loop_transport::loop_registry`. LOOP_TRANSPORT's
        // `ALONG_LOOP` clause reads back through `get_loop(name)`. The
        // FACE form always closes (face cycles round-trip the first
        // vertex by construction); the EDGES form may be open, in
        // which case the LOOP_TRANSPORT executor raises
        // `LoopNotClosed` per the gate doc §SHAM audit-story flag.
        #[cfg(feature = "gauge")]
        Statement::LoopDecl {
            name,
            lattice,
            body,
        } => {
            use crate::lattice::registry as lattice_registry;
            use crate::lattice::EdgeOrientation;
            use crate::gauge::loop_transport::{register_loop, RegisteredLoop};
            let lat = lattice_registry::get(lattice).ok_or_else(|| {
                format!("LOOP: lattice '{lattice}' is not declared")
            })?;
            let (vertices, edges) = match body {
                LoopBody::Face(face_idx) => {
                    if *face_idx >= lat.n_faces() {
                        return Err(format!(
                            "LOOP: FACE {face_idx} out of range for lattice '{lattice}' ({} faces)",
                            lat.n_faces()
                        ));
                    }
                    let face = &lat.faces[*face_idx];
                    if face.is_empty() {
                        return Err(format!("LOOP: FACE {face_idx} is empty"));
                    }
                    // Build the closed vertex path: face cycle + first
                    // vertex repeated at the end.
                    let mut vs: Vec<usize> = face.clone();
                    vs.push(face[0]);
                    let mut es: Vec<(usize, EdgeOrientation)> = Vec::with_capacity(face.len());
                    for i in 0..face.len() {
                        let a = face[i];
                        let b = face[(i + 1) % face.len()];
                        let (eid, orient) = lat.resolve_edge(a, b).ok_or_else(|| {
                            format!(
                                "LOOP: face {face_idx}: ({a},{b}) is not an edge of lattice '{lattice}'"
                            )
                        })?;
                        es.push((eid, orient));
                    }
                    (vs, es)
                }
                LoopBody::Edges(vs) => {
                    if vs.len() < 2 {
                        return Err(format!(
                            "LOOP: EDGES list needs at least 2 vertices, got {}",
                            vs.len()
                        ));
                    }
                    // Defer per-pair edge resolution to LOOP_TRANSPORT
                    // execution so the LOOP_NOT_CLOSED audit-story flag
                    // surfaces at the integrator boundary (per gate doc
                    // §SHAM). Resolution failures still bubble up — but
                    // an open EDGES path that nevertheless names valid
                    // edges registers cleanly so LOOP_TRANSPORT can
                    // raise the typed `LoopNotClosed` error itself.
                    let mut es: Vec<(usize, EdgeOrientation)> = Vec::with_capacity(vs.len() - 1);
                    for w in vs.windows(2) {
                        let a = w[0];
                        let b = w[1];
                        // Use a sentinel `(usize::MAX, Forward)` for
                        // pairs that don't resolve; LOOP_TRANSPORT
                        // doesn't walk the edge list when it detects
                        // `!is_closed()` first, so the sentinel never
                        // reaches walk_loop. The vertex path stays
                        // intact so `LoopNotClosed { tail, head }`
                        // carries the right user-facing message.
                        let (eid, orient) = lat
                            .resolve_edge(a, b)
                            .unwrap_or((usize::MAX, EdgeOrientation::Forward));
                        es.push((eid, orient));
                    }
                    (vs.clone(), es)
                }
            };
            register_loop(
                name,
                RegisteredLoop {
                    lattice_name: lattice.clone(),
                    vertices,
                    edges,
                },
            );
            Ok(ExecResult::Ok)
        }

        // ── TDD-HAL-VI.2 — LOOP_TRANSPORT executor arm ──────────────
        //
        // Per gate doc Locked decisions, the executor delegates to
        // `gauge::loop_transport::loop_transport` which reuses the
        // SYMPLECTIC_FLOW per-substep KDK building blocks
        // (wilson_force_per_edge / apply_force_kick / drift_step /
        // project_gauss / walk_loop). The Rows lowering mirrors the
        // SYMPLECTIC_FLOW arm above (single-row envelope with one
        // column per RETURN field).
        #[cfg(feature = "gauge")]
        Statement::LoopTransport { .. } => {
            // The verb mutates an SU(2) U field + companion E field
            // in place. The Statement variant doesn't carry the U/E
            // names directly (v3.1.3 §4.4 only names the LATTICE and
            // the loop_id); for the executor-dispatch path we follow
            // the convention that the GAUGE_FIELD bound to the
            // lattice is named `U_lt` and the E_FIELD `E_lt` (matches
            // the smoke test setup). This is the documented HTTP /v1
            // path's convention; direct callers (the smoke test)
            // bypass this arm and call `gauge::loop_transport`
            // directly with explicit names.
            let resp = crate::gauge::loop_transport::loop_transport(stmt, "U_lt", "E_lt")
                .map_err(|e| e.to_string())?;
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "h_forward".into(),
                crate::types::Value::Float(resp.h_forward),
            );
            record.insert(
                "h_reversed".into(),
                crate::types::Value::Float(resp.h_reversed),
            );
            record.insert(
                "sigma_h_blocked".into(),
                crate::types::Value::Float(resp.sigma_h_blocked),
            );
            record.insert(
                "per_seed_h_forward".into(),
                crate::types::Value::Vector(resp.per_seed_h_forward.clone()),
            );
            record.insert(
                "per_seed_h_reversed".into(),
                crate::types::Value::Vector(resp.per_seed_h_reversed.clone()),
            );
            record.insert(
                "tracking_error_max_q".into(),
                crate::types::Value::Float(resp.tracking_error_max_q),
            );
            record.insert(
                "tracking_error_max_beta_w".into(),
                crate::types::Value::Float(resp.tracking_error_max_beta_w),
            );
            let (verdict, ratio) = match resp.adiabaticity_check {
                crate::gauge::loop_transport::AdiabaticityCheck::Acceptable { ratio } => {
                    ("ACCEPTABLE", ratio)
                }
                crate::gauge::loop_transport::AdiabaticityCheck::AmbiguousForced { ratio } => {
                    ("AMBIGUOUS_FORCED", ratio)
                }
            };
            record.insert(
                "adiabaticity_verdict".into(),
                crate::types::Value::Text(verdict.into()),
            );
            record.insert(
                "adiabaticity_ratio".into(),
                crate::types::Value::Float(ratio),
            );
            record.insert(
                "n_substeps_completed".into(),
                crate::types::Value::Integer(resp.n_substeps_completed as i64),
            );
            Ok(ExecResult::Rows(vec![record]))
        }

        // ── TDD-HAL-V.2 — SNAPSHOT GAUGE_FIELD name PERSIST ────────
        //
        // Group-erasure dispatch (locked decision D-V-D mirrors the
        // II.5/III.6/IV.7 pattern): the parser is group-agnostic; we
        // resolve U through the dyn read registry, validate
        // `Group::SU2`, copy the buffer, and hand off to
        // `engine.snapshot_gauge_field_durable`. That helper computes
        // the SHA-256 over the LE-encoded buffer bytes (D-V-C — same
        // hash lands in the WAL and in the response envelope) and
        // appends a `WalEntry::GaugeFieldSnapshot` to the WAL.
        //
        // The response is a single-row Rows envelope carrying the
        // citation fields the Solves Vol. 4 chapter needs:
        //   { field, n_edges, repr_dim, sha256 (hex), wal_offset }.
        #[cfg(feature = "gauge")]
        Statement::Snapshot { name, persist: _ } => {
            use crate::gauge::{GaugeFieldError, Group};
            // 1. Resolve the U handle through the dyn surface. The
            //    `FieldNotDeclared` Display contains the literal
            //    "is not declared" substring the red test asserts.
            let handle = crate::gauge::registry::get(name).ok_or_else(|| {
                GaugeFieldError::FieldNotDeclared(name.clone()).to_string()
            })?;
            // 2. Group-erasure dispatch — only SU(2) proceeds at this
            //    gate. The future SU(3) / U(1) snapshots ship as new
            //    arms here (the WAL op already supports every group
            //    tag per V.1's LE encoding).
            if !matches!(handle.group(), Group::SU2) {
                return Err(GaugeFieldError::UnsupportedGroup(handle.group()).to_string());
            }
            let buf = handle.as_dense_buffer();
            let n_edges = buf.n_edges;
            let repr_dim = buf.repr_dim;
            // 3. Hand off to the engine. The engine method does the
            //    SHA-256 + WAL-write + offset bookkeeping and returns
            //    the citation handle. The buffer clone is one
            //    `Vec<f64>` of length `n_edges * repr_dim` (2880 bytes
            //    of `f64` data for SU(2) on the buckyball).
            let buffer = buf.data.clone();
            let resp = engine
                .snapshot_gauge_field_durable(name, handle.group(), buffer)
                .map_err(|e| format!("gauge: SNAPSHOT PERSIST failed: {e}"))?;
            // 4. Lower into the Rows envelope. SHA-256 is emitted as
            //    lowercase hex (the canonical citation form).
            let mut record: crate::types::Record = std::collections::HashMap::new();
            record.insert(
                "field".into(),
                crate::types::Value::Text(name.clone()),
            );
            record.insert(
                "n_edges".into(),
                crate::types::Value::Integer(n_edges as i64),
            );
            record.insert(
                "repr_dim".into(),
                crate::types::Value::Integer(repr_dim as i64),
            );
            record.insert(
                "sha256".into(),
                crate::types::Value::Text(resp.sha256),
            );
            record.insert(
                "wal_offset".into(),
                crate::types::Value::Integer(resp.wal_offset as i64),
            );
            Ok(ExecResult::Rows(vec![record]))
        }
    }
}

impl MeasureSpec {
    pub fn func_name(&self) -> &str {
        match self.func {
            AggFunc::Count => "count",
            AggFunc::Sum => "sum",
            AggFunc::Avg => "avg",
            AggFunc::Min => "min",
            AggFunc::Max => "max",
        }
    }
}

fn filter_columns(record: crate::types::Record, columns: &[SelectCol]) -> crate::types::Record {
    if columns.iter().any(|c| matches!(c, SelectCol::Star)) {
        return record;
    }
    let mut filtered = HashMap::new();
    for col in columns {
        if let SelectCol::Name(name) = col {
            if let Some(v) = record.get(name) {
                filtered.insert(name.clone(), v.clone());
            }
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SQL compat tests (existing) ──

    #[test]
    fn parse_create_bundle() {
        let stmt = parse("CREATE BUNDLE employees (id INT BASE, name TEXT FIBER, salary FLOAT RANGE(100000) FIBER INDEX)").unwrap();
        match stmt {
            Statement::CreateBundle {
                name,
                base_fields,
                fiber_fields,
                indexed,
                ..
            } => {
                assert_eq!(name, "employees");
                assert_eq!(base_fields.len(), 1);
                assert_eq!(base_fields[0].name, "id");
                assert_eq!(fiber_fields.len(), 2);
                assert_eq!(fiber_fields[1].name, "salary");
                assert_eq!(fiber_fields[1].range, Some(100000.0));
                assert_eq!(indexed, vec!["salary"]);
            }
            _ => panic!("Expected CreateBundle"),
        }
    }

    #[test]
    fn parse_insert() {
        let stmt =
            parse("INSERT INTO employees (id, name, salary) VALUES (1, 'Alice', 75000.0)").unwrap();
        match stmt {
            Statement::Insert {
                bundle,
                columns,
                values,
            } => {
                assert_eq!(bundle, "employees");
                assert_eq!(columns, vec!["id", "name", "salary"]);
                assert_eq!(values[0], Literal::Integer(1));
                assert_eq!(values[1], Literal::Text("Alice".into()));
                assert_eq!(values[2], Literal::Integer(75000));
            }
            _ => panic!("Expected Insert"),
        }
    }

    #[test]
    fn parse_select_point_query() {
        let stmt = parse("SELECT * FROM employees WHERE id = 1").unwrap();
        match stmt {
            Statement::Select {
                bundle,
                columns,
                condition,
                group_by,
            } => {
                assert_eq!(bundle, "employees");
                assert_eq!(columns, vec![SelectCol::Star]);
                assert_eq!(
                    condition,
                    Some(Condition::Eq("id".into(), Literal::Integer(1)))
                );
                assert!(group_by.is_none());
            }
            _ => panic!("Expected Select"),
        }
    }

    #[test]
    fn parse_select_range() {
        let stmt =
            parse("SELECT name, salary FROM employees WHERE salary BETWEEN 50000 AND 100000")
                .unwrap();
        match stmt {
            Statement::Select { condition, .. } => {
                assert_eq!(
                    condition,
                    Some(Condition::Between(
                        "salary".into(),
                        Literal::Integer(50000),
                        Literal::Integer(100000)
                    ))
                );
            }
            _ => panic!("Expected Select"),
        }
    }

    #[test]
    fn parse_select_group_by() {
        let stmt = parse("SELECT dept, AVG(salary) FROM employees GROUP BY dept").unwrap();
        match stmt {
            Statement::Select {
                columns, group_by, ..
            } => {
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0], SelectCol::Name("dept".into()));
                assert_eq!(columns[1], SelectCol::Agg(AggFunc::Avg, "salary".into()));
                assert_eq!(group_by, Some("dept".into()));
            }
            _ => panic!("Expected Select"),
        }
    }

    #[test]
    fn parse_join() {
        let stmt = parse("SELECT * FROM orders JOIN customers ON customer_id").unwrap();
        match stmt {
            Statement::Join {
                left,
                right,
                on_field,
                ..
            } => {
                assert_eq!(left, "orders");
                assert_eq!(right, "customers");
                assert_eq!(on_field, "customer_id");
            }
            _ => panic!("Expected Join"),
        }
    }

    #[test]
    fn parse_curvature_spectral() {
        assert!(matches!(
            parse("CURVATURE employees").unwrap(),
            Statement::Curvature { .. }
        ));
        assert!(matches!(
            parse("SPECTRAL employees").unwrap(),
            Statement::Spectral { .. }
        ));
    }

    /// Cognitive Geometry Correspondence (Branch VII — Davis 2026-05-29).
    /// Tests that the three new GQL verbs parse correctly.
    #[test]
    fn parse_cognitive_geometry_verbs() {
        // CAPACITY: default τ=1.0
        match parse("CAPACITY sensors").unwrap() {
            Statement::Capacity { bundle, tau } => {
                assert_eq!(bundle, "sensors");
                assert!((tau - 1.0).abs() < f64::EPSILON, "default τ should be 1.0");
            }
            _ => panic!("Expected Capacity"),
        }
        // CAPACITY: explicit τ
        match parse("CAPACITY sensors TOLERANCE 2.5").unwrap() {
            Statement::Capacity { bundle, tau } => {
                assert_eq!(bundle, "sensors");
                assert!((tau - 2.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Capacity with TOLERANCE"),
        }
        // HORIZON: default τ=1.0, no config
        match parse("HORIZON sensors").unwrap() {
            Statement::Horizon { bundle, tau, config } => {
                assert_eq!(bundle, "sensors");
                assert!((tau - 1.0).abs() < f64::EPSILON);
                assert!(config.is_none());
            }
            _ => panic!("Expected Horizon"),
        }
        // HORIZON: explicit τ, no config
        match parse("HORIZON sensors TOLERANCE 3.0").unwrap() {
            Statement::Horizon { bundle, tau, config } => {
                assert_eq!(bundle, "sensors");
                assert!((tau - 3.0).abs() < f64::EPSILON);
                assert!(config.is_none());
            }
            _ => panic!("Expected Horizon with TOLERANCE"),
        }
        // HORIZON: LENGTH_SCALE WELFORD_RADIUS override
        match parse("HORIZON sensors LENGTH_SCALE WELFORD_RADIUS").unwrap() {
            Statement::Horizon { bundle, tau: _, config } => {
                assert_eq!(bundle, "sensors");
                let c = config.expect("LENGTH_SCALE supplied → config Some");
                assert_eq!(c.estimator, crate::curvature::LengthScaleEstimator::WelfordRadius);
            }
            _ => panic!("Expected Horizon with LENGTH_SCALE"),
        }
        // HORIZON: LENGTH_SCALE FIXED <n> override
        match parse("HORIZON sensors TOLERANCE 2.0 LENGTH_SCALE FIXED 3.14").unwrap() {
            Statement::Horizon { bundle, tau, config } => {
                assert_eq!(bundle, "sensors");
                assert!((tau - 2.0).abs() < 1e-12);
                let c = config.expect("config Some");
                assert!(matches!(
                    c.estimator,
                    crate::curvature::LengthScaleEstimator::Fixed(v) if (v - 3.14).abs() < 1e-12
                ));
            }
            _ => panic!("Expected Horizon with FIXED"),
        }
        // DEPTH: no overrides → config is None (executor will use defaults)
        match parse("DEPTH sensors").unwrap() {
            Statement::Depth { bundle, config } => {
                assert_eq!(bundle, "sensors");
                assert!(config.is_none(), "no overrides supplied → config should be None");
            }
            _ => panic!("Expected Depth"),
        }

        // DEPTH: single override (lambda1_topological = 0 — the JTBD-demo
        // fix for sensor bundles where spectral_gap returns ~0)
        match parse("DEPTH sensors LAMBDA1_TOPOLOGICAL 0").unwrap() {
            Statement::Depth { bundle, config } => {
                assert_eq!(bundle, "sensors");
                let c = config.expect("override supplied → config should be Some");
                assert!((c.lambda1_topological - 0.0).abs() < f64::EPSILON);
                // Other fields keep defaults
                assert!((c.k_metric - 0.5).abs() < f64::EPSILON);
                assert!((c.k_connection - 0.1).abs() < f64::EPSILON);
                assert!((c.lambda1_connection - 0.3).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Depth"),
        }

        // DEPTH: all four overrides, mixed order
        match parse(
            "DEPTH sensors K_METRIC 2.0 LAMBDA1_CONNECTION 0.05 \
             K_CONNECTION 0.25 LAMBDA1_TOPOLOGICAL 0.0001",
        )
        .unwrap()
        {
            Statement::Depth { bundle, config } => {
                assert_eq!(bundle, "sensors");
                let c = config.expect("overrides supplied");
                assert!((c.k_metric - 2.0).abs() < 1e-12);
                assert!((c.k_connection - 0.25).abs() < 1e-12);
                assert!((c.lambda1_topological - 0.0001).abs() < 1e-12);
                assert!((c.lambda1_connection - 0.05).abs() < 1e-12);
            }
            _ => panic!("Expected Depth"),
        }
    }

    /// Round-trip execute for cognitive geometry verbs on a real bundle.
    #[test]
    fn execute_cognitive_geometry_verbs() {
        let dir = std::env::temp_dir().join("gigi_cog_geo_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = crate::engine::Engine::open(&dir).unwrap();

        execute(&mut engine,
            &parse("CREATE BUNDLE cog (x INT BASE, y FLOAT FIBER)").unwrap()
        ).unwrap();
        for i in 0..8 {
            execute(&mut engine,
                &parse(&format!("INSERT INTO cog (x, y) VALUES ({i}, {})", i as f64 * 0.5)).unwrap()
            ).unwrap();
        }

        // CAPACITY should return a finite or infinite scalar
        let cap = execute(&mut engine, &parse("CAPACITY cog").unwrap()).unwrap();
        assert!(matches!(cap, ExecResult::Scalar(_)), "CAPACITY returned non-scalar");

        // HORIZON should return a finite or infinite scalar
        let hor = execute(&mut engine, &parse("HORIZON cog").unwrap()).unwrap();
        assert!(matches!(hor, ExecResult::Scalar(_)), "HORIZON returned non-scalar");

        // DEPTH should return 1.0–4.0
        if let ExecResult::Scalar(level) = execute(&mut engine, &parse("DEPTH cog").unwrap()).unwrap() {
            assert!(level >= 1.0 && level <= 4.0, "DEPTH level out of range: {level}");
        } else {
            panic!("DEPTH returned non-scalar");
        }
    }

    #[test]
    fn execute_full_workflow() {
        let dir = std::env::temp_dir().join("gigi_parser_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = crate::engine::Engine::open(&dir).unwrap();

        // Create bundle
        let stmt = parse("CREATE BUNDLE emp (id INT BASE, name TEXT FIBER, salary FLOAT RANGE(100000) FIBER INDEX)").unwrap();
        execute(&mut engine, &stmt).unwrap();

        // Insert
        for i in 0..5 {
            let sql = format!(
                "INSERT INTO emp (id, name, salary) VALUES ({i}, 'Person{i}', {})",
                50000.0 + i as f64 * 10000.0
            );
            let stmt = parse(&sql).unwrap();
            execute(&mut engine, &stmt).unwrap();
        }

        // Point query
        let stmt = parse("SELECT * FROM emp WHERE id = 0").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 1),
            _ => panic!("Expected rows"),
        }

        // Full scan
        let stmt = parse("SELECT * FROM emp").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 5),
            _ => panic!("Expected rows"),
        }

        // Curvature
        let stmt = parse("CURVATURE emp").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        assert!(matches!(result, ExecResult::Scalar(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GQL Native tests ──

    #[test]
    fn gql_bundle_keyword_style() {
        let stmt = parse("BUNDLE sensors BASE (id NUMERIC) FIBER (city CATEGORICAL INDEX, temp NUMERIC RANGE 80)").unwrap();
        match stmt {
            Statement::CreateBundle {
                name,
                base_fields,
                fiber_fields,
                indexed,
                ..
            } => {
                assert_eq!(name, "sensors");
                assert_eq!(base_fields.len(), 1);
                assert_eq!(base_fields[0].name, "id");
                assert_eq!(fiber_fields.len(), 2);
                assert_eq!(fiber_fields[0].name, "city");
                assert_eq!(fiber_fields[1].range, Some(80.0));
                assert_eq!(indexed, vec!["city"]);
            }
            _ => panic!("Expected CreateBundle"),
        }
    }

    #[test]
    fn parse_invariant_clause() {
        let stmt = parse("BUNDLE quat BASE (id NUMERIC) FIBER (w NUMERIC, x NUMERIC, y NUMERIC, z NUMERIC) ADJACENCY () INVARIANT norm = 1.0 +/- 0.01").unwrap();
        match stmt {
            Statement::CreateBundle { name, invariants, .. } => {
                assert_eq!(name, "quat");
                assert_eq!(invariants.len(), 1);
                assert_eq!(invariants[0].field, "norm");
                assert!((invariants[0].expected - 1.0).abs() < 1e-9);
                assert!((invariants[0].tol - 0.01).abs() < 1e-9);
            }
            _ => panic!("Expected CreateBundle"),
        }
    }

    #[test]
    fn gql_section_insert() {
        let stmt = parse("SECTION sensors (id: 42, city: 'Moscow', temp: -31.9)").unwrap();
        match stmt {
            Statement::Insert {
                bundle,
                columns,
                values,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(columns, vec!["id", "city", "temp"]);
                assert_eq!(values[0], Literal::Integer(42));
                assert_eq!(values[1], Literal::Text("Moscow".into()));
            }
            _ => panic!("Expected Insert"),
        }
    }

    #[test]
    fn gql_section_upsert() {
        let stmt = parse("SECTION sensors (id: 42, city: 'Moscow', temp: -28.5) UPSERT").unwrap();
        assert!(matches!(stmt, Statement::SectionUpsert { .. }));
    }

    #[test]
    fn gql_section_point_query() {
        let stmt = parse("SECTION sensors AT id=42").unwrap();
        match stmt {
            Statement::PointQuery {
                bundle,
                key,
                project,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(key, vec![("id".into(), Literal::Integer(42))]);
                assert!(project.is_none());
            }
            _ => panic!("Expected PointQuery"),
        }
    }

    #[test]
    fn gql_section_projected() {
        let stmt = parse("SECTION sensors AT id=42 PROJECT (city, temp)").unwrap();
        match stmt {
            Statement::PointQuery { project, .. } => {
                assert_eq!(project, Some(vec!["city".into(), "temp".into()]));
            }
            _ => panic!("Expected PointQuery"),
        }
    }

    #[test]
    fn gql_exists_section() {
        let stmt = parse("EXISTS SECTION sensors AT id=42").unwrap();
        assert!(matches!(stmt, Statement::ExistsSection { .. }));
    }

    #[test]
    fn gql_redefine_point() {
        let stmt = parse("REDEFINE sensors AT id=42 SET (temp: -28.5)").unwrap();
        match stmt {
            Statement::Redefine { bundle, key, sets } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(key[0], ("id".into(), Literal::Integer(42)));
                assert_eq!(sets[0].0, "temp");
            }
            _ => panic!("Expected Redefine"),
        }
    }

    #[test]
    fn gql_retract() {
        let stmt = parse("RETRACT sensors AT id=42").unwrap();
        match stmt {
            Statement::Retract { bundle, key } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(key[0], ("id".into(), Literal::Integer(42)));
            }
            _ => panic!("Expected Retract"),
        }
    }

    #[test]
    fn gql_cover_on() {
        let stmt = parse("COVER sensors ON city = 'Moscow'").unwrap();
        match stmt {
            Statement::Cover {
                bundle,
                on_conditions,
                ..
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(on_conditions.len(), 1);
                assert_eq!(
                    on_conditions[0],
                    FilterCondition::Eq("city".into(), Literal::Text("Moscow".into()))
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_where_lt() {
        let stmt = parse("COVER sensors WHERE temp < -25").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(where_conditions.len(), 1);
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Lt("temp".into(), Literal::Integer(-25))
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_on_where_combined() {
        let stmt = parse("COVER sensors ON city = 'Moscow' WHERE temp < -25").unwrap();
        match stmt {
            Statement::Cover {
                on_conditions,
                where_conditions,
                ..
            } => {
                assert_eq!(on_conditions.len(), 1);
                assert_eq!(where_conditions.len(), 1);
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_distinct() {
        let stmt = parse("COVER sensors DISTINCT city").unwrap();
        match stmt {
            Statement::Cover { distinct_field, .. } => {
                assert_eq!(distinct_field, Some("city".into()));
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_rank_first_skip() {
        let stmt = parse("COVER sensors RANK BY temp DESC SKIP 10 FIRST 5").unwrap();
        match stmt {
            Statement::Cover {
                rank_by,
                skip,
                first,
                ..
            } => {
                let sort = rank_by.unwrap();
                assert_eq!(sort[0].field, "temp");
                assert!(sort[0].desc);
                assert_eq!(skip, Some(10));
                assert_eq!(first, Some(5));
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_all() {
        let stmt = parse("COVER sensors ALL").unwrap();
        match stmt {
            Statement::Cover { all, .. } => assert!(all),
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_in() {
        let stmt = parse("COVER sensors ON region IN ('EU', 'NA')").unwrap();
        match stmt {
            Statement::Cover { on_conditions, .. } => {
                assert_eq!(
                    on_conditions[0],
                    FilterCondition::In(
                        "region".into(),
                        vec![Literal::Text("EU".into()), Literal::Text("NA".into())]
                    )
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_void_defined() {
        let stmt = parse("COVER sensors WHERE pressure VOID").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Void("pressure".into())
                );
            }
            _ => panic!("Expected Cover"),
        }
        let stmt = parse("COVER sensors WHERE pressure DEFINED").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Defined("pressure".into())
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_matches() {
        let stmt = parse("COVER sensors WHERE city MATCHES 'Mos*'").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Matches("city".into(), "Mos*".into())
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_project() {
        let stmt = parse("COVER sensors ON city = 'Moscow' PROJECT (city, temp, wind)").unwrap();
        match stmt {
            Statement::Cover { project, .. } => {
                assert_eq!(
                    project,
                    Some(vec!["city".into(), "temp".into(), "wind".into()])
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_integrate_over_measure() {
        let stmt = parse("INTEGRATE sensors OVER city MEASURE avg(temp), count(*)").unwrap();
        match stmt {
            Statement::Integrate {
                bundle,
                over,
                measures,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(over, Some("city".into()));
                assert_eq!(measures.len(), 2);
                assert_eq!(measures[0].field, "temp");
                assert!(matches!(measures[0].func, AggFunc::Avg));
                assert_eq!(measures[1].field, "*");
                assert!(matches!(measures[1].func, AggFunc::Count));
            }
            _ => panic!("Expected Integrate"),
        }
    }

    #[test]
    fn gql_integrate_global() {
        let stmt = parse("INTEGRATE sensors MEASURE avg(temp), max(wind)").unwrap();
        match stmt {
            Statement::Integrate { over, measures, .. } => {
                assert!(over.is_none());
                assert_eq!(measures.len(), 2);
            }
            _ => panic!("Expected Integrate"),
        }
    }

    /// Regression: COUNT(*) in INTEGRATE...MEASURE returned 0 because the
    /// executor filtered records by `r.get("*")` (always None) before
    /// counting. Found by the GIGI Builds book harness for chapter 1
    /// (151-record stations bundle: sum_temp and avg_temp correct,
    /// count_* = 0).
    #[test]
    fn gql_integrate_count_star_returns_record_count() {
        use crate::engine::Engine;
        use crate::types::{BundleSchema, FieldDef, Record, Value};

        let dir = std::env::temp_dir().join("gigi_integrate_count_star_regression");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut engine = Engine::open(&dir).unwrap();
        engine
            .create_bundle(
                BundleSchema::new("stations")
                    .base(FieldDef::categorical("station_id"))
                    .fiber(FieldDef::numeric("temp").with_range(100.0)),
            )
            .unwrap();
        for i in 0..151 {
            let mut r = Record::new();
            r.insert("station_id".into(), Value::Text(format!("s{i:03}")));
            r.insert("temp".into(), Value::Float(20.0 + (i % 7) as f64 * 0.1));
            engine.insert("stations", &r).unwrap();
        }

        let stmt =
            parse("INTEGRATE stations MEASURE SUM(temp), AVG(temp), COUNT(*)").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        let rows = match result {
            ExecResult::Rows(rs) => rs,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        let count_star = row
            .get("count_*")
            .expect("count_* column missing from row");
        assert_eq!(
            count_star,
            &Value::Float(151.0),
            "COUNT(*) should be 151, got {count_star:?}"
        );
        // SUM/AVG must remain correct after the fix. 151 records with
        // temp = 20.0 + (i % 7) * 0.1: 21 full mod-7 cycles (deviation
        // sum 2.1 each = 44.1) plus i%7=0..3 (0.6) = 44.7; plus
        // 151 * 20.0 = 3020 ⇒ expected sum 3064.7, avg ≈ 20.296.
        let expected_sum = 3064.7;
        match row.get("sum_temp") {
            Some(Value::Float(v)) => {
                assert!(
                    (*v - expected_sum).abs() < 0.001,
                    "sum_temp = {v}, expected ≈ {expected_sum}"
                );
            }
            other => panic!("sum_temp missing or wrong type: {other:?}"),
        }
        match row.get("avg_temp") {
            Some(Value::Float(v)) => {
                let expected_avg = expected_sum / 151.0;
                assert!(
                    (*v - expected_avg).abs() < 0.001,
                    "avg_temp = {v}, expected ≈ {expected_avg}"
                );
            }
            other => panic!("avg_temp missing or wrong type: {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Regression: COUNT(*) in INTEGRATE ... OVER ... MEASURE returned
    /// zero rows on populated bundles, because the executor passed "*"
    /// as agg_field to group_by, which skipped every record (rec.get("*")
    /// is always None). Three subtests covering the failure modes:
    /// (1) COUNT(*) alone — single measure with no real field;
    /// (2) COUNT(*) MIXED with SUM/AVG — COUNT(*) must count every
    ///     record in each group, not just records with non-null agg_field;
    /// (3) COUNT(*) when some records have null agg_field — COUNT(*)
    ///     still reflects the whole group.
    #[test]
    fn gql_integrate_over_count_star_returns_records_per_group() {
        use crate::engine::Engine;
        use crate::types::{BundleSchema, FieldDef, Record, Value};

        let dir = std::env::temp_dir()
            .join("gigi_integrate_over_count_star_regression");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut engine = Engine::open(&dir).unwrap();
        engine
            .create_bundle(
                BundleSchema::new("sales")
                    .base(FieldDef::categorical("id"))
                    .fiber(FieldDef::categorical("region"))
                    .fiber(FieldDef::numeric("amount").with_range(1000.0)),
            )
            .unwrap();
        // 6 north + 4 south + 2 east = 12 records.
        let layout = [("north", 6), ("south", 4), ("east", 2)];
        let mut next_id = 0;
        for (region, n) in &layout {
            for j in 0..*n {
                let mut r = Record::new();
                r.insert("id".into(), Value::Text(format!("r{next_id:03}")));
                r.insert("region".into(), Value::Text((*region).into()));
                r.insert(
                    "amount".into(),
                    Value::Float(100.0 + j as f64 * 10.0),
                );
                engine.insert("sales", &r).unwrap();
                next_id += 1;
            }
        }

        let into_map =
            |rs: Vec<Record>| -> std::collections::HashMap<String, Record> {
                rs.into_iter()
                    .map(|r| {
                        let key = match r.get("region") {
                            Some(Value::Text(s)) => s.clone(),
                            other => panic!("unexpected region key: {other:?}"),
                        };
                        (key, r)
                    })
                    .collect()
            };

        // (1) Single COUNT(*) measure — agg_field falls back to "*".
        let stmt = parse("INTEGRATE sales OVER region MEASURE COUNT(*)")
            .unwrap();
        let rows = match execute(&mut engine, &stmt).unwrap() {
            ExecResult::Rows(rs) => rs,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(
            rows.len(),
            3,
            "expected one row per region, got {}",
            rows.len()
        );
        let by_region = into_map(rows);
        for (region, expected) in &layout {
            let row = by_region
                .get(*region)
                .unwrap_or_else(|| panic!("missing region {region}"));
            assert_eq!(
                row.get("count_*"),
                Some(&Value::Float(*expected as f64)),
                "COUNT(*) wrong for region {region}",
            );
        }

        // (2) Mixed measures — COUNT(*) must still count every record
        //     even when a different field drives SUM/AVG.
        let stmt = parse(
            "INTEGRATE sales OVER region MEASURE COUNT(*), SUM(amount), AVG(amount)",
        )
        .unwrap();
        let rows = match execute(&mut engine, &stmt).unwrap() {
            ExecResult::Rows(rs) => rs,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 3, "expected 3 region rows in mixed-measure");
        let by_region = into_map(rows);
        for (region, expected_n) in &layout {
            let row = by_region
                .get(*region)
                .unwrap_or_else(|| panic!("missing region {region}"));
            assert_eq!(
                row.get("count_*"),
                Some(&Value::Float(*expected_n as f64)),
                "COUNT(*) wrong for region {region} in mixed-measure",
            );
            let expected_sum: f64 =
                (0..*expected_n).map(|j| 100.0 + j as f64 * 10.0).sum();
            match row.get("sum_amount") {
                Some(Value::Float(v)) => assert!(
                    (*v - expected_sum).abs() < 0.001,
                    "SUM(amount) for {region}: {v} vs expected {expected_sum}"
                ),
                other => panic!("sum_amount missing for {region}: {other:?}"),
            }
            match row.get("avg_amount") {
                Some(Value::Float(v)) => assert!(
                    (*v - expected_sum / *expected_n as f64).abs() < 0.001,
                    "AVG(amount) wrong for {region}",
                ),
                other => panic!("avg_amount missing for {region}: {other:?}"),
            }
        }

        // (3) A record with explicit null in the agg_field still gets
        //     counted by COUNT(*) in its group.
        let mut r = Record::new();
        r.insert("id".into(), Value::Text("rNULL".into()));
        r.insert("region".into(), Value::Text("east".into()));
        r.insert("amount".into(), Value::Null);
        engine.insert("sales", &r).unwrap();

        let stmt = parse(
            "INTEGRATE sales OVER region MEASURE COUNT(*), SUM(amount)",
        )
        .unwrap();
        let rows = match execute(&mut engine, &stmt).unwrap() {
            ExecResult::Rows(rs) => rs,
            other => panic!("expected Rows, got {other:?}"),
        };
        let by_region = into_map(rows);
        let east = by_region.get("east").expect("east row missing");
        assert_eq!(
            east.get("count_*"),
            Some(&Value::Float(3.0)),
            "east COUNT(*) must include the null-amount record (2 + 1 = 3)",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gql_pullback() {
        let stmt = parse("PULLBACK readings ALONG sensor_id ONTO sensors").unwrap();
        match stmt {
            Statement::Pullback {
                left, along, right, ..
            } => {
                assert_eq!(left, "readings");
                assert_eq!(along, "sensor_id");
                assert_eq!(right, "sensors");
            }
            _ => panic!("Expected Pullback"),
        }
    }

    #[test]
    fn gql_pullback_preserve_left() {
        let stmt = parse("PULLBACK readings ALONG sensor_id ONTO sensors PRESERVE LEFT").unwrap();
        match stmt {
            Statement::Pullback { preserve_left, .. } => assert!(preserve_left),
            _ => panic!("Expected Pullback"),
        }
    }

    #[test]
    fn gql_curvature_fields_by() {
        let stmt = parse("CURVATURE sensors ON temp, wind BY city").unwrap();
        match stmt {
            Statement::Curvature {
                bundle,
                fields,
                by_field,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(fields, vec!["temp", "wind"]);
                assert_eq!(by_field, Some("city".into()));
            }
            _ => panic!("Expected Curvature"),
        }
    }

    #[test]
    fn gql_spectral_full() {
        let stmt = parse("SPECTRAL sensors FULL").unwrap();
        match stmt {
            Statement::Spectral { bundle, full } => {
                assert_eq!(bundle, "sensors");
                assert!(full);
            }
            _ => panic!("Expected Spectral"),
        }
    }

    #[test]
    fn gql_consistency_repair() {
        let stmt = parse("CONSISTENCY sensors REPAIR").unwrap();
        match stmt {
            Statement::Consistency { bundle, repair } => {
                assert_eq!(bundle, "sensors");
                assert!(repair);
            }
            _ => panic!("Expected Consistency"),
        }
    }

    #[test]
    fn gql_suggest_adjacency_basic() {
        let stmt = parse("SUGGEST_ADJACENCY ON chembl_activities MINIMIZING h1").unwrap();
        match stmt {
            Statement::SuggestAdjacency {
                bundle,
                fields,
                sample_size,
                candidates,
            } => {
                assert_eq!(bundle, "chembl_activities");
                assert!(fields.is_empty());
                assert_eq!(sample_size, 10_000);
                assert_eq!(candidates, 5);
            }
            _ => panic!("Expected SuggestAdjacency"),
        }
    }

    #[test]
    fn gql_suggest_adjacency_full() {
        let stmt = parse(
            "SUGGEST_ADJACENCY ON mydata FIELDS pchembl_value, assay_type SAMPLE_SIZE 5000 CANDIDATES 10 MINIMIZING h1",
        )
        .unwrap();
        match stmt {
            Statement::SuggestAdjacency {
                bundle,
                fields,
                sample_size,
                candidates,
            } => {
                assert_eq!(bundle, "mydata");
                assert_eq!(fields, vec!["pchembl_value", "assay_type"]);
                assert_eq!(sample_size, 5000);
                assert_eq!(candidates, 10);
            }
            _ => panic!("Expected SuggestAdjacency"),
        }
    }

    #[test]
    fn gql_show_bundles() {
        assert!(matches!(
            parse("SHOW BUNDLES").unwrap(),
            Statement::ShowBundles
        ));
    }

    #[test]
    fn gql_describe() {
        let stmt = parse("DESCRIBE sensors VERBOSE").unwrap();
        match stmt {
            Statement::Describe { bundle, verbose } => {
                assert_eq!(bundle, "sensors");
                assert!(verbose);
            }
            _ => panic!("Expected Describe"),
        }
    }

    #[test]
    fn gql_collapse() {
        let stmt = parse("COLLAPSE sensors").unwrap();
        match stmt {
            Statement::Collapse { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected Collapse"),
        }
    }

    #[test]
    fn gql_health() {
        let stmt = parse("HEALTH sensors").unwrap();
        assert!(matches!(stmt, Statement::Health { .. }));
    }

    #[test]
    fn gql_explain() {
        let stmt = parse("EXPLAIN COVER sensors ON city = 'Moscow'").unwrap();
        match stmt {
            Statement::Explain { inner } => {
                assert!(matches!(*inner, Statement::Cover { .. }));
            }
            _ => panic!("Expected Explain"),
        }
    }

    #[test]
    fn gql_atlas_begin_commit() {
        assert!(matches!(
            parse("ATLAS BEGIN").unwrap(),
            Statement::AtlasBegin
        ));
        assert!(matches!(
            parse("ATLAS COMMIT").unwrap(),
            Statement::AtlasCommit
        ));
        assert!(matches!(
            parse("ATLAS ROLLBACK").unwrap(),
            Statement::AtlasRollback
        ));
    }

    #[test]
    fn gql_bare_transaction_verbs() {
        // Bare spellings promised by ATOMIC_SHEAF_COMMIT_SPEC §3.5/§7.2;
        // aliases for the ATLAS transaction-control statements.
        assert!(matches!(parse("BEGIN").unwrap(), Statement::AtlasBegin));
        assert!(matches!(
            parse("BEGIN TRANSACTION").unwrap(),
            Statement::AtlasBegin
        ));
        assert!(matches!(parse("COMMIT").unwrap(), Statement::AtlasCommit));
        assert!(matches!(
            parse("ROLLBACK").unwrap(),
            Statement::AtlasRollback
        ));
    }

    #[test]
    fn gql_cover_between() {
        let stmt = parse("COVER sensors WHERE temp BETWEEN -30 AND 0").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Between(
                        "temp".into(),
                        Literal::Integer(-30),
                        Literal::Integer(0)
                    )
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_not_in() {
        let stmt = parse("COVER sensors WHERE region NOT IN ('TEST', 'DEV')").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::NotIn(
                        "region".into(),
                        vec![Literal::Text("TEST".into()), Literal::Text("DEV".into())]
                    )
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_line_comment_ignored() {
        let stmt = parse("-- this is a comment\nSHOW BUNDLES").unwrap();
        assert!(matches!(stmt, Statement::ShowBundles));
    }

    #[test]
    fn gql_execute_full_workflow() {
        let dir = std::env::temp_dir().join("gigi_gql_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = crate::engine::Engine::open(&dir).unwrap();

        // BUNDLE (keyword style)
        let stmt = parse("BUNDLE emp BASE (id NUMERIC) FIBER (name CATEGORICAL, salary NUMERIC RANGE 100000 INDEX, dept CATEGORICAL INDEX)").unwrap();
        execute(&mut engine, &stmt).unwrap();

        // SECTION (insert)
        for i in 0..5 {
            let gql = format!(
                "SECTION emp (id: {i}, name: 'Person{i}', salary: {}, dept: 'Eng')",
                50000 + i * 10000
            );
            execute(&mut engine, &parse(&gql).unwrap()).unwrap();
        }

        // SECTION AT (point query)
        let result = execute(&mut engine, &parse("SECTION emp AT id=0").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(
                    rows[0].get("name"),
                    Some(&crate::types::Value::Text("Person0".into()))
                );
            }
            _ => panic!("Expected rows"),
        }

        // SECTION AT ... PROJECT
        let result = execute(
            &mut engine,
            &parse("SECTION emp AT id=0 PROJECT (name, salary)").unwrap(),
        )
        .unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2); // only name + salary
            }
            _ => panic!("Expected rows"),
        }

        // EXISTS SECTION
        let result = execute(&mut engine, &parse("EXISTS SECTION emp AT id=0").unwrap()).unwrap();
        assert_eq!(result, ExecResult::Bool(true));
        let result = execute(&mut engine, &parse("EXISTS SECTION emp AT id=999").unwrap()).unwrap();
        assert_eq!(result, ExecResult::Bool(false));

        // COVER ALL
        let result = execute(&mut engine, &parse("COVER emp ALL").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 5),
            _ => panic!("Expected rows"),
        }

        // COVER ON (bitmap query)
        let result = execute(&mut engine, &parse("COVER emp ON dept = 'Eng'").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 5),
            _ => panic!("Expected rows"),
        }

        // COVER WHERE (filter query)
        let result = execute(
            &mut engine,
            &parse("COVER emp WHERE salary > 70000").unwrap(),
        )
        .unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 2), // 80000, 90000
            _ => panic!("Expected rows"),
        }

        // COVER DISTINCT
        let result = execute(&mut engine, &parse("COVER emp DISTINCT dept").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 1), // just "Eng"
            _ => panic!("Expected rows"),
        }

        // REDEFINE (update)
        execute(
            &mut engine,
            &parse("REDEFINE emp AT id=0 SET (salary: 99000)").unwrap(),
        )
        .unwrap();
        let result = execute(&mut engine, &parse("SECTION emp AT id=0").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(
                    rows[0].get("salary"),
                    Some(&crate::types::Value::Integer(99000))
                );
            }
            _ => panic!("Expected rows"),
        }

        // RETRACT (delete)
        execute(&mut engine, &parse("RETRACT emp AT id=4").unwrap()).unwrap();
        let result = execute(&mut engine, &parse("COVER emp ALL").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 4),
            _ => panic!("Expected rows"),
        }

        // INTEGRATE (aggregation)
        let result = execute(
            &mut engine,
            &parse("INTEGRATE emp OVER dept MEASURE avg(salary), count(*)").unwrap(),
        )
        .unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(rows.len(), 1); // one group: "Eng"
                assert!(rows[0].contains_key("dept"));
            }
            _ => panic!("Expected rows"),
        }

        // CURVATURE
        let result = execute(&mut engine, &parse("CURVATURE emp").unwrap()).unwrap();
        assert!(matches!(result, ExecResult::Scalar(_)));

        // SPECTRAL
        let result = execute(&mut engine, &parse("SPECTRAL emp").unwrap()).unwrap();
        assert!(matches!(result, ExecResult::Scalar(_)));

        // SHOW BUNDLES
        let result = execute(&mut engine, &parse("SHOW BUNDLES").unwrap()).unwrap();
        match result {
            ExecResult::Bundles(infos) => {
                assert_eq!(infos.len(), 1);
                assert_eq!(infos[0].name, "emp");
            }
            _ => panic!("Expected Bundles"),
        }

        // DESCRIBE
        let result = execute(&mut engine, &parse("DESCRIBE emp").unwrap()).unwrap();
        match result {
            ExecResult::Stats(stats) => {
                assert_eq!(stats.record_count, 4);
                assert_eq!(stats.base_fields, 1);
                assert_eq!(stats.fiber_fields, 3);
            }
            _ => panic!("Expected Stats"),
        }

        // HEALTH
        let result = execute(&mut engine, &parse("HEALTH emp").unwrap()).unwrap();
        assert!(matches!(result, ExecResult::Stats(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GQL v2.1 tests ──

    #[test]
    fn gql_weave_role() {
        let stmt = parse("WEAVE ROLE analyst PASSWORD 'hash123' INHERITS reader").unwrap();
        match stmt {
            Statement::WeaveRole {
                name,
                password,
                inherits,
                superweave,
            } => {
                assert_eq!(name, "analyst");
                assert_eq!(password, Some("hash123".into()));
                assert_eq!(inherits, Some("reader".into()));
                assert!(!superweave);
            }
            _ => panic!("Expected WeaveRole"),
        }
    }

    #[test]
    fn gql_weave_role_superweave() {
        let stmt = parse("WEAVE ROLE admin SUPERWEAVE").unwrap();
        match stmt {
            Statement::WeaveRole {
                name, superweave, ..
            } => {
                assert_eq!(name, "admin");
                assert!(superweave);
            }
            _ => panic!("Expected WeaveRole"),
        }
    }

    #[test]
    fn gql_unweave_role() {
        let stmt = parse("UNWEAVE ROLE analyst").unwrap();
        match stmt {
            Statement::UnweaveRole { name } => assert_eq!(name, "analyst"),
            _ => panic!("Expected UnweaveRole"),
        }
    }

    #[test]
    fn gql_show_roles() {
        assert!(matches!(parse("SHOW ROLES").unwrap(), Statement::ShowRoles));
    }

    #[test]
    fn gql_grant() {
        let stmt = parse("GRANT COVER, INTEGRATE ON sensors TO analyst").unwrap();
        match stmt {
            Statement::Grant {
                operations,
                bundle,
                role,
            } => {
                assert_eq!(operations, vec!["COVER", "INTEGRATE"]);
                assert_eq!(bundle, "sensors");
                assert_eq!(role, "analyst");
            }
            _ => panic!("Expected Grant"),
        }
    }

    #[test]
    fn gql_revoke() {
        let stmt = parse("REVOKE RETRACT ON sensors FROM analyst").unwrap();
        match stmt {
            Statement::Revoke {
                operations,
                bundle,
                role,
            } => {
                assert_eq!(operations, vec!["RETRACT"]);
                assert_eq!(bundle, "sensors");
                assert_eq!(role, "analyst");
            }
            _ => panic!("Expected Revoke"),
        }
    }

    #[test]
    fn gql_drop_policy() {
        let stmt = parse("DROP POLICY region_restrict ON sensors").unwrap();
        match stmt {
            Statement::DropPolicy { name, bundle } => {
                assert_eq!(name, "region_restrict");
                assert_eq!(bundle, "sensors");
            }
            _ => panic!("Expected DropPolicy"),
        }
    }

    #[test]
    fn gql_audit_on() {
        let stmt = parse("AUDIT sensors ON").unwrap();
        match stmt {
            Statement::AuditOn { bundle, operations } => {
                assert_eq!(bundle, "sensors");
                assert!(operations.is_empty());
            }
            _ => panic!("Expected AuditOn"),
        }
    }

    #[test]
    fn gql_audit_off() {
        let stmt = parse("AUDIT sensors OFF").unwrap();
        match stmt {
            Statement::AuditOff { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected AuditOff"),
        }
    }

    #[test]
    fn gql_audit_show() {
        let stmt = parse("AUDIT SHOW sensors SINCE '2024-01-01' ROLE admin").unwrap();
        match stmt {
            Statement::AuditShow {
                bundle,
                since,
                role,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(since, Some("2024-01-01".into()));
                assert_eq!(role, Some("admin".into()));
            }
            _ => panic!("Expected AuditShow"),
        }
    }

    #[test]
    fn gql_gauge_constrain() {
        let stmt =
            parse("GAUGE orders CONSTRAIN (ADD CHECK (total > 0) AS positive_total)").unwrap();
        match stmt {
            Statement::GaugeConstrain {
                bundle,
                constraints,
            } => {
                assert_eq!(bundle, "orders");
                assert_eq!(constraints.len(), 1);
                assert!(constraints[0].contains("CHECK"));
            }
            _ => panic!("Expected GaugeConstrain"),
        }
    }

    #[test]
    fn gql_gauge_unconstrain() {
        let stmt = parse("GAUGE orders UNCONSTRAIN positive_total").unwrap();
        match stmt {
            Statement::GaugeUnconstrain {
                bundle,
                constraint_name,
            } => {
                assert_eq!(bundle, "orders");
                assert_eq!(constraint_name, "positive_total");
            }
            _ => panic!("Expected GaugeUnconstrain"),
        }
    }

    #[test]
    fn gql_show_constraints() {
        let stmt = parse("SHOW CONSTRAINTS ON sensors").unwrap();
        match stmt {
            Statement::ShowConstraints { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowConstraints"),
        }
    }

    #[test]
    fn gql_compact() {
        let stmt = parse("COMPACT sensors ANALYZE").unwrap();
        match stmt {
            Statement::Compact { bundle, analyze } => {
                assert_eq!(bundle, "sensors");
                assert!(analyze);
            }
            _ => panic!("Expected Compact"),
        }
    }

    #[test]
    fn gql_analyze() {
        let stmt = parse("ANALYZE sensors FULL").unwrap();
        match stmt {
            Statement::Analyze {
                bundle,
                field,
                full,
            } => {
                assert_eq!(bundle, "sensors");
                assert!(field.is_none());
                assert!(full);
            }
            _ => panic!("Expected Analyze"),
        }
    }

    #[test]
    fn gql_analyze_field() {
        let stmt = parse("ANALYZE sensors ON temp").unwrap();
        match stmt {
            Statement::Analyze {
                bundle,
                field,
                full,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, Some("temp".into()));
                assert!(!full);
            }
            _ => panic!("Expected Analyze"),
        }
    }

    #[test]
    fn gql_vacuum() {
        let stmt = parse("VACUUM sensors FULL").unwrap();
        match stmt {
            Statement::Vacuum { bundle, full } => {
                assert_eq!(bundle, "sensors");
                assert!(full);
            }
            _ => panic!("Expected Vacuum"),
        }
    }

    #[test]
    fn gql_rebuild_index() {
        let stmt = parse("REBUILD INDEX sensors ON city").unwrap();
        match stmt {
            Statement::RebuildIndex { bundle, field } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, Some("city".into()));
            }
            _ => panic!("Expected RebuildIndex"),
        }
    }

    #[test]
    fn gql_check_integrity() {
        let stmt = parse("CHECK sensors").unwrap();
        match stmt {
            Statement::CheckIntegrity { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected CheckIntegrity"),
        }
    }

    #[test]
    fn gql_repair() {
        let stmt = parse("REPAIR sensors").unwrap();
        match stmt {
            Statement::Repair { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected Repair"),
        }
    }

    #[test]
    fn gql_storage() {
        let stmt = parse("STORAGE sensors").unwrap();
        match stmt {
            Statement::StorageInfo { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected StorageInfo"),
        }
    }

    #[test]
    fn gql_set() {
        let stmt = parse("SET TOLERANCE 0.01").unwrap();
        match stmt {
            Statement::Set { key, value } => {
                assert_eq!(key, "TOLERANCE");
                assert_eq!(value, Literal::Float(0.01));
            }
            _ => panic!("Expected Set"),
        }
    }

    #[test]
    fn gql_reset() {
        assert!(matches!(
            parse("RESET ALL").unwrap(),
            Statement::Reset { key: None }
        ));
        let stmt = parse("RESET TOLERANCE").unwrap();
        match stmt {
            Statement::Reset { key } => assert_eq!(key, Some("TOLERANCE".into())),
            _ => panic!("Expected Reset"),
        }
    }

    #[test]
    fn gql_show_settings() {
        assert!(matches!(
            parse("SHOW SETTINGS").unwrap(),
            Statement::ShowSettings
        ));
    }

    #[test]
    fn gql_show_session() {
        assert!(matches!(
            parse("SHOW SESSION").unwrap(),
            Statement::ShowSession
        ));
    }

    #[test]
    fn gql_show_current_role() {
        let stmt = parse("SHOW CURRENT ROLE").unwrap();
        assert!(matches!(stmt, Statement::ShowCurrentRole));
    }

    #[test]
    fn gql_ingest() {
        let stmt = parse("INGEST sensors FROM 'data.csv' FORMAT CSV").unwrap();
        match stmt {
            Statement::Ingest {
                bundle,
                source,
                format,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(source, "data.csv");
                assert_eq!(format, "CSV");
            }
            _ => panic!("Expected Ingest"),
        }
    }

    #[test]
    fn gql_ingest_stdin() {
        let stmt = parse("INGEST sensors FROM STDIN FORMAT JSONL").unwrap();
        match stmt {
            Statement::Ingest { source, format, .. } => {
                assert_eq!(source, "STDIN");
                assert_eq!(format, "JSONL");
            }
            _ => panic!("Expected Ingest"),
        }
    }

    #[test]
    fn gql_transplant() {
        let stmt =
            parse("TRANSPLANT sensors INTO sensors_archive WHERE date < 20240101 RETRACT SOURCE")
                .unwrap();
        match stmt {
            Statement::Transplant {
                source,
                target,
                conditions,
                retract_source,
            } => {
                assert_eq!(source, "sensors");
                assert_eq!(target, "sensors_archive");
                assert_eq!(conditions.len(), 1);
                assert!(retract_source);
            }
            _ => panic!("Expected Transplant"),
        }
    }

    #[test]
    fn gql_generate_base() {
        let stmt =
            parse("GENERATE BASE sensors FROM date=20240101 TO date=20241231 STEP 1").unwrap();
        match stmt {
            Statement::GenerateBase {
                bundle,
                field,
                from_val,
                to_val,
                step,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, "date");
                assert_eq!(from_val, Literal::Integer(20240101));
                assert_eq!(to_val, Literal::Integer(20241231));
                assert_eq!(step, Literal::Integer(1));
            }
            _ => panic!("Expected GenerateBase"),
        }
    }

    #[test]
    fn gql_fill() {
        let stmt = parse("FILL sensors ON date USING INTERPOLATE LINEAR").unwrap();
        match stmt {
            Statement::Fill {
                bundle,
                field,
                method,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, "date");
                assert_eq!(method, "INTERPOLATE LINEAR");
            }
            _ => panic!("Expected Fill"),
        }
    }

    #[test]
    fn gql_prepare() {
        let stmt =
            parse("PREPARE city_query AS COVER sensors ON city = $1 WHERE temp < $2").unwrap();
        match stmt {
            Statement::Prepare { name, body } => {
                assert_eq!(name, "city_query");
                assert!(body.contains("COVER"));
                assert!(body.contains("sensors"));
            }
            _ => panic!("Expected Prepare"),
        }
    }

    #[test]
    fn gql_execute_params() {
        let stmt = parse("EXECUTE city_query ('Moscow', -25)").unwrap();
        match stmt {
            Statement::Execute { name, params } => {
                assert_eq!(name, "city_query");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], Literal::Text("Moscow".into()));
                assert_eq!(params[1], Literal::Integer(-25));
            }
            _ => panic!("Expected Execute"),
        }
    }

    #[test]
    fn gql_deallocate() {
        assert!(matches!(
            parse("DEALLOCATE ALL").unwrap(),
            Statement::Deallocate { name: None }
        ));
        let stmt = parse("DEALLOCATE city_query").unwrap();
        match stmt {
            Statement::Deallocate { name } => assert_eq!(name, Some("city_query".into())),
            _ => panic!("Expected Deallocate"),
        }
    }

    #[test]
    fn gql_show_prepared() {
        assert!(matches!(
            parse("SHOW PREPARED").unwrap(),
            Statement::ShowPrepared
        ));
    }

    #[test]
    fn gql_backup() {
        let stmt = parse("BACKUP sensors TO 'sensors_2024.gigi' COMPRESS").unwrap();
        match stmt {
            Statement::Backup {
                bundle,
                path,
                compress,
                incremental_since,
            } => {
                assert_eq!(bundle, Some("sensors".into()));
                assert_eq!(path, "sensors_2024.gigi");
                assert!(compress);
                assert!(incremental_since.is_none());
            }
            _ => panic!("Expected Backup"),
        }
    }

    #[test]
    fn gql_backup_all() {
        let stmt = parse("BACKUP ALL TO 'full.gigi'").unwrap();
        match stmt {
            Statement::Backup { bundle, path, .. } => {
                assert!(bundle.is_none());
                assert_eq!(path, "full.gigi");
            }
            _ => panic!("Expected Backup"),
        }
    }

    #[test]
    fn gql_backup_incremental() {
        let stmt = parse("BACKUP sensors TO 'incr.gigi' INCREMENTAL SINCE '2024-06-01'").unwrap();
        match stmt {
            Statement::Backup {
                incremental_since, ..
            } => {
                assert_eq!(incremental_since, Some("2024-06-01".into()));
            }
            _ => panic!("Expected Backup"),
        }
    }

    #[test]
    fn gql_restore() {
        let stmt = parse("RESTORE sensors FROM 'sensors_2024.gigi' AS sensors_restored").unwrap();
        match stmt {
            Statement::Restore {
                bundle,
                path,
                snapshot,
                rename,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(path, "sensors_2024.gigi");
                assert!(snapshot.is_none());
                assert_eq!(rename, Some("sensors_restored".into()));
            }
            _ => panic!("Expected Restore"),
        }
    }

    #[test]
    fn gql_restore_snapshot() {
        let stmt = parse("RESTORE sensors FROM 'backup.gigi' AT SNAPSHOT 'pre_migration'").unwrap();
        match stmt {
            Statement::Restore { snapshot, .. } => {
                assert_eq!(snapshot, Some("pre_migration".into()));
            }
            _ => panic!("Expected Restore"),
        }
    }

    #[test]
    fn gql_verify_backup() {
        let stmt = parse("VERIFY BACKUP 'sensors_2024.gigi'").unwrap();
        match stmt {
            Statement::VerifyBackup { path } => assert_eq!(path, "sensors_2024.gigi"),
            _ => panic!("Expected VerifyBackup"),
        }
    }

    #[test]
    fn gql_show_backups() {
        assert!(matches!(
            parse("SHOW BACKUPS").unwrap(),
            Statement::ShowBackups
        ));
    }

    #[test]
    fn gql_show_fields() {
        let stmt = parse("SHOW FIELDS ON sensors").unwrap();
        match stmt {
            Statement::ShowFields { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowFields"),
        }
    }

    #[test]
    fn gql_show_indexes() {
        let stmt = parse("SHOW INDEXES ON sensors").unwrap();
        match stmt {
            Statement::ShowIndexes { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowIndexes"),
        }
    }

    #[test]
    fn gql_show_morphisms() {
        let stmt = parse("SHOW MORPHISMS ON orders").unwrap();
        match stmt {
            Statement::ShowMorphisms { bundle } => assert_eq!(bundle, "orders"),
            _ => panic!("Expected ShowMorphisms"),
        }
    }

    #[test]
    fn gql_show_triggers() {
        let stmt = parse("SHOW TRIGGERS ON sensors").unwrap();
        match stmt {
            Statement::ShowTriggers { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowTriggers"),
        }
    }

    #[test]
    fn gql_show_policies() {
        let stmt = parse("SHOW POLICIES ON sensors").unwrap();
        match stmt {
            Statement::ShowPolicies { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowPolicies"),
        }
    }

    #[test]
    fn gql_show_statistics() {
        let stmt = parse("SHOW STATISTICS ON sensors").unwrap();
        match stmt {
            Statement::ShowStatistics { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowStatistics"),
        }
    }

    #[test]
    fn gql_show_geometry() {
        let stmt = parse("SHOW GEOMETRY ON sensors").unwrap();
        match stmt {
            Statement::ShowGeometry { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowGeometry"),
        }
    }

    #[test]
    fn gql_show_comments() {
        let stmt = parse("SHOW COMMENTS ON sensors").unwrap();
        match stmt {
            Statement::ShowComments { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowComments"),
        }
    }

    #[test]
    fn gql_comment_on_bundle() {
        let stmt = parse("COMMENT ON BUNDLE sensors IS 'NASA atmospheric data'").unwrap();
        match stmt {
            Statement::CommentOn {
                target_type,
                target,
                comment,
            } => {
                assert_eq!(target_type, "BUNDLE");
                assert_eq!(target, "sensors");
                assert_eq!(comment, "NASA atmospheric data");
            }
            _ => panic!("Expected CommentOn"),
        }
    }

    #[test]
    fn gql_comment_on_field() {
        let stmt = parse("COMMENT ON FIELD sensors.temp IS 'Temperature at 2m'").unwrap();
        match stmt {
            Statement::CommentOn {
                target_type,
                target,
                comment,
            } => {
                assert_eq!(target_type, "FIELD");
                assert_eq!(target, "sensors.temp");
                assert_eq!(comment, "Temperature at 2m");
            }
            _ => panic!("Expected CommentOn"),
        }
    }

    #[test]
    fn gql_iterate() {
        let stmt =
            parse("ITERATE employees START AT id=1 STEP ALONG manager_id UNTIL VOID MAX DEPTH 10")
                .unwrap();
        match stmt {
            Statement::Iterate {
                bundle,
                start_key,
                step_field,
                max_depth,
            } => {
                assert_eq!(bundle, "employees");
                assert_eq!(start_key, vec![("id".into(), Literal::Integer(1))]);
                assert_eq!(step_field, "manager_id");
                assert_eq!(max_depth, Some(10));
            }
            _ => panic!("Expected Iterate"),
        }
    }

    #[test]
    fn gql_iterate_no_depth() {
        let stmt =
            parse("ITERATE friends START AT user_id=42 STEP ALONG friend_id UNTIL VOID").unwrap();
        match stmt {
            Statement::Iterate {
                bundle,
                step_field,
                max_depth,
                ..
            } => {
                assert_eq!(bundle, "friends");
                assert_eq!(step_field, "friend_id");
                assert!(max_depth.is_none());
            }
            _ => panic!("Expected Iterate"),
        }
    }

    #[test]
    fn gql_iterate_depth_only() {
        let stmt = parse("ITERATE friends START AT user_id=42 STEP ALONG friend_id UNTIL DEPTH 3")
            .unwrap();
        match stmt {
            Statement::Iterate { max_depth, .. } => {
                assert_eq!(max_depth, Some(3));
            }
            _ => panic!("Expected Iterate"),
        }
    }

    #[test]
    fn gql_drop_trigger() {
        let stmt = parse("DROP TRIGGER extreme_cold ON sensors").unwrap();
        match stmt {
            Statement::DropTrigger { name, bundle } => {
                assert_eq!(name, "extreme_cold");
                assert_eq!(bundle, "sensors");
            }
            _ => panic!("Expected DropTrigger"),
        }
    }

    // ── PERCEIVE GQL parser ────────────────────────────────────

    /// Identity rotation in 2D, vector unchanged. Verifies the
    /// happy-path GQL syntax: PERCEIVE <bundle> ROTATION (...) VECTOR (...).
    #[test]
    fn gql_perceive_identity_2d() {
        let stmt = parse(
            "PERCEIVE my_bundle ROTATION (1.0, 0.0, 0.0, 1.0) VECTOR (3.0, 4.0)",
        )
        .unwrap();
        match stmt {
            Statement::Perceive { bundle, rotation, vector, dim } => {
                assert_eq!(bundle, "my_bundle");
                assert_eq!(rotation, vec![1.0, 0.0, 0.0, 1.0]);
                assert_eq!(vector, vec![3.0, 4.0]);
                assert_eq!(dim, None, "DIM omitted ⇒ inferred at execute time");
            }
            _ => panic!("Expected Perceive, got {:?}", stmt),
        }
    }

    /// Explicit DIM keyword overrides the inference. Useful when the
    /// rotation length encodes a dim the caller wants to validate.
    #[test]
    fn gql_perceive_with_explicit_dim() {
        let stmt = parse(
            "PERCEIVE sensors ROTATION (0.0, -1.0, 1.0, 0.0) VECTOR (1.0, 0.0) DIM 2",
        )
        .unwrap();
        match stmt {
            Statement::Perceive { dim, .. } => assert_eq!(dim, Some(2)),
            _ => panic!("Expected Perceive"),
        }
    }

    /// Clauses can appear in any order. Pin both orderings.
    #[test]
    fn gql_perceive_clause_order_flexible() {
        // VECTOR before ROTATION
        let a = parse(
            "PERCEIVE b VECTOR (1.0, 2.0) ROTATION (1.0, 0.0, 0.0, 1.0)",
        )
        .unwrap();
        let b = parse(
            "PERCEIVE b ROTATION (1.0, 0.0, 0.0, 1.0) VECTOR (1.0, 2.0)",
        )
        .unwrap();
        // Same parsed Statement either way.
        match (a, b) {
            (
                Statement::Perceive {
                    rotation: ra,
                    vector: va,
                    ..
                },
                Statement::Perceive {
                    rotation: rb,
                    vector: vb,
                    ..
                },
            ) => {
                assert_eq!(ra, rb);
                assert_eq!(va, vb);
            }
            _ => panic!("Expected Perceive variants"),
        }
    }

    /// Missing ROTATION clause is a parser error with a clear message.
    /// The user-facing error path matters here — wrong matrix input is
    /// the most common GQL user mistake on this verb.
    #[test]
    fn gql_perceive_missing_rotation_is_an_error() {
        let err = parse("PERCEIVE b VECTOR (1.0, 0.0)").unwrap_err();
        assert!(
            err.contains("ROTATION"),
            "error should mention ROTATION, got: {}",
            err
        );
    }

    /// Missing VECTOR clause is a parser error.
    #[test]
    fn gql_perceive_missing_vector_is_an_error() {
        let err = parse("PERCEIVE b ROTATION (1.0, 0.0, 0.0, 1.0)").unwrap_err();
        assert!(
            err.contains("VECTOR"),
            "error should mention VECTOR, got: {}",
            err
        );
    }

    /// Non-integer DIM is rejected at parse time (catches typos like
    /// `DIM 3.5` before they reach the executor).
    #[test]
    fn gql_perceive_non_integer_dim_rejected() {
        let err = parse(
            "PERCEIVE b ROTATION (1.0, 0.0, 0.0, 1.0) VECTOR (1.0, 0.0) DIM 2.5",
        )
        .unwrap_err();
        assert!(
            err.contains("DIM"),
            "error should mention DIM, got: {}",
            err
        );
    }

    #[test]
    fn gql_on_trigger() {
        let stmt = parse("ON SECTION sensors EXECUTE NOTIFY 'new_reading'").unwrap();
        match stmt {
            Statement::CreateTrigger {
                event,
                bundle,
                condition,
                action,
            } => {
                assert_eq!(event, "ON SECTION");
                assert_eq!(bundle, "sensors");
                assert!(condition.is_none());
                assert!(action.contains("NOTIFY"));
            }
            _ => panic!("Expected CreateTrigger"),
        }
    }

    #[test]
    fn gql_on_trigger_with_condition() {
        let stmt =
            parse("ON SECTION sensors WHERE temp < -30 EXECUTE ALERT 'extreme_cold'").unwrap();
        match stmt {
            Statement::CreateTrigger {
                event,
                bundle,
                condition,
                action,
            } => {
                assert_eq!(event, "ON SECTION");
                assert_eq!(bundle, "sensors");
                assert!(condition.is_some());
                assert!(condition.unwrap().contains("temp"));
                assert!(action.contains("ALERT"));
            }
            _ => panic!("Expected CreateTrigger"),
        }
    }

    #[test]
    fn gql_sections_column_list_tuples() {
        let stmt =
            parse("SECTIONS sensors (id, city, temp) (1, 'Moscow', -27.1), (2, 'Berlin', 5.0)")
                .unwrap();
        match stmt {
            Statement::BatchInsert {
                bundle,
                columns,
                rows,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(columns, vec!["id", "city", "temp"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 3);
                assert_eq!(rows[1].len(), 3);
            }
            _ => panic!("Expected BatchInsert"),
        }
    }

    #[test]
    fn gql_sections_column_list_single_tuple() {
        let stmt = parse("SECTIONS s (a, b) (1, 'x')").unwrap();
        match stmt {
            Statement::BatchInsert {
                bundle,
                columns,
                rows,
            } => {
                assert_eq!(bundle, "s");
                assert_eq!(columns, vec!["a", "b"]);
                assert_eq!(rows.len(), 1);
            }
            _ => panic!("Expected BatchInsert"),
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // GIGI Encrypt v0.2 — Sprint A: per-field encryption mode declaration.
    //
    // Tests for the GQL surface defined in GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md §3.1.
    // Verify that `CREATE BUNDLE` accepts per-field `ENCRYPTED [MODE]` clauses,
    // wires them through to FieldDef::encryption, validates type-mode
    // compatibility, and preserves v0.1 backwards compat.
    // ════════════════════════════════════════════════════════════════════════

    use crate::types::EncryptionMode;

    /// Helper: parse a CREATE BUNDLE stmt and return the FieldSpec list (base + fiber).
    fn parse_create_bundle_specs(sql: &str) -> (Vec<FieldSpec>, Vec<FieldSpec>) {
        let stmt = parse(sql).unwrap_or_else(|e| panic!("parse failed for {sql}: {e}"));
        match stmt {
            Statement::CreateBundle {
                base_fields, fiber_fields, ..
            } => (base_fields, fiber_fields),
            _ => panic!("Expected CreateBundle"),
        }
    }

    /// Helper: pick a fiber field by name out of a parsed CreateBundle.
    fn fiber_field<'a>(fiber: &'a [FieldSpec], name: &str) -> &'a FieldSpec {
        fiber
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("no fiber field named {name}"))
    }

    #[test]
    fn test_parse_create_bundle_field_level_opaque() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE acct (id INT BASE, legal_name TEXT FIBER ENCRYPTED OPAQUE)",
        );
        assert_eq!(fiber_field(&fiber, "legal_name").encryption, EncryptionMode::Opaque);
    }

    #[test]
    fn test_parse_create_bundle_field_level_indexed() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE evt (id INT BASE, kind TEXT FIBER ENCRYPTED INDEXED)",
        );
        assert_eq!(fiber_field(&fiber, "kind").encryption, EncryptionMode::Indexed);
    }

    #[test]
    fn test_parse_create_bundle_field_level_probabilistic_with_sigma() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE evt (id INT BASE, amount NUMERIC FIBER ENCRYPTED PROBABILISTIC SIGMA 0.5)",
        );
        match fiber_field(&fiber, "amount").encryption {
            EncryptionMode::Probabilistic { sigma } => assert!((sigma - 0.5).abs() < 1e-12),
            other => panic!("expected Probabilistic, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_create_bundle_field_level_isometric_numeric() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE wind (sid INT BASE, wx NUMERIC FIBER ENCRYPTED ISOMETRIC)",
        );
        assert_eq!(fiber_field(&fiber, "wx").encryption, EncryptionMode::Isometric);
    }

    #[test]
    fn test_parse_create_bundle_field_level_affine() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED AFFINE)",
        );
        assert_eq!(fiber_field(&fiber, "t").encryption, EncryptionMode::Affine);
    }

    #[test]
    fn test_parse_create_bundle_default_mode_for_text_is_opaque() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE acct (id INT BASE, legal_name TEXT FIBER ENCRYPTED)",
        );
        assert_eq!(fiber_field(&fiber, "legal_name").encryption, EncryptionMode::Opaque);
    }

    #[test]
    fn test_parse_create_bundle_default_mode_for_numeric_is_affine() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED)",
        );
        assert_eq!(fiber_field(&fiber, "t").encryption, EncryptionMode::Affine);
    }

    #[test]
    fn test_parse_create_bundle_v01_compat_no_mode() {
        // v0.1 syntax: bundle-level ENCRYPTED with no per-field clause.
        // Per-field encryption stays None at parse time; the engine fills it
        // in during CreateBundle dispatch using `default_for_type`.
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE m (id INT BASE, t FLOAT FIBER) ENCRYPTED",
        );
        assert_eq!(fiber_field(&fiber, "t").encryption, EncryptionMode::None);
    }

    #[test]
    fn test_parse_unencrypted_field_is_none() {
        let (_b, fiber) = parse_create_bundle_specs(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER)",
        );
        assert_eq!(fiber_field(&fiber, "t").encryption, EncryptionMode::None);
    }

    #[test]
    fn test_parse_rejects_probabilistic_on_text() {
        let result = parse(
            "CREATE BUNDLE m (id INT BASE, label TEXT FIBER ENCRYPTED PROBABILISTIC SIGMA 0.1)",
        );
        let err = result.expect_err("PROBABILISTIC on TEXT should be rejected");
        assert!(
            err.to_lowercase().contains("probabilistic"),
            "error should mention PROBABILISTIC, got: {err}"
        );
    }

    #[test]
    fn test_parse_rejects_indexed_on_numeric() {
        let result = parse(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED INDEXED)",
        );
        let err = result.expect_err("INDEXED on NUMERIC should be rejected");
        assert!(
            err.to_lowercase().contains("indexed"),
            "error should mention INDEXED, got: {err}"
        );
    }

    #[test]
    fn test_parse_rejects_affine_on_text() {
        let result = parse(
            "CREATE BUNDLE m (id INT BASE, label TEXT FIBER ENCRYPTED AFFINE)",
        );
        let err = result.expect_err("AFFINE on TEXT should be rejected");
        assert!(
            err.to_lowercase().contains("affine"),
            "error should mention AFFINE, got: {err}"
        );
    }

    #[test]
    fn test_parse_sigma_value_required_with_probabilistic() {
        let result = parse(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED PROBABILISTIC)",
        );
        let err = result.expect_err("PROBABILISTIC without SIGMA should be rejected");
        assert!(
            err.to_lowercase().contains("sigma"),
            "error should mention SIGMA, got: {err}"
        );
    }

    #[test]
    fn test_parse_sigma_must_be_positive() {
        let result = parse(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED PROBABILISTIC SIGMA 0)",
        );
        let err = result.expect_err("SIGMA 0 should be rejected");
        assert!(
            err.to_lowercase().contains("sigma") || err.to_lowercase().contains("positive"),
            "error should mention SIGMA / positive, got: {err}"
        );

        let result = parse(
            "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED PROBABILISTIC SIGMA -1)",
        );
        result.expect_err("negative SIGMA should be rejected");
    }

    // ── Sprint F: WITH ENCRYPTION SEED clause ──

  #[test]
  fn test_parse_with_encryption_seed_hex() {
      let hex = "0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c";
      let stmt = parse(&format!(
          "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED) WITH ENCRYPTION SEED '{hex}'"
      )).unwrap();
      match stmt {
          Statement::CreateBundle { seed_source, .. } => {
              match seed_source {
                  crate::types::EncryptionSeedSource::Hex(s) => assert_eq!(s, hex),
                  other => panic!("expected Hex seed source, got {:?}", other),
              }
          }
          _ => panic!("Expected CreateBundle"),
      }
  }

  #[test]
  fn test_parse_seed_hex_must_be_64_chars() {
      let result = parse(
          "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER) WITH ENCRYPTION SEED 'tooshort'"
      );
      let err = result.expect_err("64-char check should fire");
      assert!(err.contains("64") || err.to_lowercase().contains("characters"),
          "error should mention the 64-char rule, got: {err}");
  }

  #[test]
  fn test_parse_seed_hex_rejects_non_hex_chars() {
      // 64 chars but contains non-hex 'g'.
      let bad = "g".repeat(64);
      let result = parse(&format!(
          "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER) WITH ENCRYPTION SEED '{bad}'"
      ));
      let err = result.expect_err("non-hex should be rejected");
      assert!(err.to_lowercase().contains("hex"),
          "error should mention hex constraint, got: {err}");
  }

  #[test]
  fn test_parse_seed_from_env() {
      let stmt = parse(
          "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED) WITH ENCRYPTION SEED FROM ENV JG_GIGI_SEED"
      ).unwrap();
      match stmt {
          Statement::CreateBundle { seed_source, .. } => {
              match seed_source {
                  crate::types::EncryptionSeedSource::Env(n) => assert_eq!(n, "JG_GIGI_SEED"),
                  other => panic!("expected Env seed source, got {:?}", other),
              }
          }
          _ => panic!("Expected CreateBundle"),
      }
  }

  #[test]
  fn test_parse_no_seed_clause_defaults_to_random() {
      let stmt = parse(
          "CREATE BUNDLE m (id INT BASE, t NUMERIC FIBER ENCRYPTED)"
      ).unwrap();
      match stmt {
          Statement::CreateBundle { seed_source, .. } => {
              assert_eq!(seed_source, crate::types::EncryptionSeedSource::Random);
          }
          _ => panic!("Expected CreateBundle"),
      }
  }

  #[test]
  fn test_parse_seed_clause_with_per_field_modes() {
      // The realistic case: per-field modes + user-supplied hex seed.
      let hex = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
      let sql = format!(
          "CREATE BUNDLE acct (\
            email TEXT BASE, \
            legal_name TEXT FIBER ENCRYPTED OPAQUE, \
            kind TEXT FIBER ENCRYPTED INDEXED\
          ) WITH ENCRYPTION SEED '{hex}'"
      );
      let stmt = parse(&sql).unwrap();
      match stmt {
          Statement::CreateBundle { seed_source, fiber_fields, .. } => {
              assert!(matches!(seed_source, crate::types::EncryptionSeedSource::Hex(_)));
              assert_eq!(fiber_fields[0].encryption, EncryptionMode::Opaque);
              assert_eq!(fiber_fields[1].encryption, EncryptionMode::Indexed);
          }
          _ => panic!("Expected CreateBundle"),
      }
  }

  #[test]
  fn test_parse_mixed_modes_in_one_bundle() {
        // The realistic case: jg_account-style schema with a mix of opaque text
        // fields, indexed text fields, and a numeric field. Every per-field
        // mode is declared explicitly; bundle-level ENCRYPTED is NOT used.
        let sql = "CREATE BUNDLE acct (\
            email TEXT BASE, \
            legal_name TEXT FIBER ENCRYPTED OPAQUE, \
            kind TEXT FIBER ENCRYPTED INDEXED, \
            score NUMERIC FIBER ENCRYPTED AFFINE, \
            attempts INT FIBER\
        )";
        let (_b, fiber) = parse_create_bundle_specs(sql);
        assert_eq!(fiber_field(&fiber, "legal_name").encryption, EncryptionMode::Opaque);
        assert_eq!(fiber_field(&fiber, "kind").encryption, EncryptionMode::Indexed);
        assert_eq!(fiber_field(&fiber, "score").encryption, EncryptionMode::Affine);
        assert_eq!(fiber_field(&fiber, "attempts").encryption, EncryptionMode::None);
    }

    /// C2 — TRANSPORT_ROTATION must parse and populate
    /// Statement::TransportRotation with FROM/TO/ON FIBER/WITH ANGLE.
    #[test]
    fn test_parse_transport_rotation() {
        let sql = "TRANSPORT_ROTATION emb FROM (id='a') TO (id='b') \
                   ON FIBER (f0, f1, f2, f3) WITH ANGLE 0.785";
        let stmt = parse(sql).expect("parse failed");
        match stmt {
            Statement::TransportRotation {
                bundle, from_keys, to_keys, fiber_fields, angle,
            } => {
                assert_eq!(bundle, "emb");
                assert_eq!(from_keys.len(), 1);
                assert_eq!(to_keys.len(), 1);
                assert_eq!(
                    fiber_fields,
                    vec!["f0".to_string(), "f1".to_string(),
                         "f2".to_string(), "f3".to_string()],
                );
                assert!((angle - 0.785).abs() < 1e-9);
            }
            other => panic!("Expected TransportRotation, got {other:?}"),
        }
    }

    // ─── TDD-HAL-II.5 — GAUGE_FIELD parser + executor ───────────────
    //
    // Closes the parser + embedded-executor half of GAUGE_FIELD
    // declaration. Bee's locked decisions wired through:
    //   1. (CSPRNG / intra-binding bit-identity) — SU2GaugeField::new
    //      runs the Marsaglia sampler through GIGI's SmallRng, so
    //      `INIT HAAR_RANDOM SEED N` produces a buffer determined
    //      entirely by the seed.
    //   3. (PERSIST routing) — `PERSIST` keyword routes the executor
    //      through `engine.declare_gauge_field_durable`; without it
    //      the registration is in-memory.
    //   5. (typed errors) — every failure mode surfaces as a
    //      `GaugeFieldError` Display string; Halcyon's regex anchors
    //      `SU\(2\)` / `SEED` / `not declared` all match.
    //   6. (group erasure) — the parser accepts SU(2)/SU(3)/U(1)/Z(N)
    //      syntactically; only Group::SU2 proceeds to construction.

    /// TDD-HAL-II.5: `GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2)
    /// INIT IDENTITY;` parses into the expected Statement::GaugeField
    /// shape (the red anchor for the gate).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_parse_identity() {
        let stmt = parse(
            "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;",
        )
        .expect("parse GAUGE_FIELD identity");
        match stmt {
            Statement::GaugeField {
                name,
                lattice,
                group,
                init,
                seed,
                persist,
            } => {
                assert_eq!(name, "U");
                assert_eq!(lattice, "buckyball");
                assert_eq!(group, crate::gauge::Group::SU2);
                assert_eq!(init, crate::gauge::GaugeFieldInit::Identity);
                assert_eq!(seed, None);
                assert!(!persist);
            }
            other => panic!("Expected Statement::GaugeField, got {other:?}"),
        }
    }

    /// TDD-HAL-II.5: HAAR_RANDOM with SEED + PERSIST round-trips
    /// through the parser into the expected Statement::GaugeField
    /// shape.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_parse_haar_with_seed() {
        let stmt = parse(
            "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) \
             INIT HAAR_RANDOM SEED 20260616 PERSIST;",
        )
        .expect("parse GAUGE_FIELD haar + persist");
        match stmt {
            Statement::GaugeField {
                name,
                lattice,
                group,
                init,
                seed,
                persist,
            } => {
                assert_eq!(name, "U");
                assert_eq!(lattice, "buckyball");
                assert_eq!(group, crate::gauge::Group::SU2);
                assert_eq!(init, crate::gauge::GaugeFieldInit::HaarRandom);
                assert_eq!(seed, Some(20260616));
                assert!(persist);
            }
            other => panic!("Expected Statement::GaugeField, got {other:?}"),
        }
    }

    /// TDD-HAL-II.5: `INIT FROM other_field` round-trips into a
    /// `GaugeFieldInit::FromField("other_field")` variant. The source
    /// is resolved by name at executor time.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_parse_from_field() {
        let stmt = parse(
            "GAUGE_FIELD U2 ON LATTICE buckyball GROUP SU(2) INIT FROM other_field;",
        )
        .expect("parse GAUGE_FIELD INIT FROM");
        match stmt {
            Statement::GaugeField {
                name,
                lattice,
                group,
                init,
                seed,
                persist,
            } => {
                assert_eq!(name, "U2");
                assert_eq!(lattice, "buckyball");
                assert_eq!(group, crate::gauge::Group::SU2);
                assert_eq!(
                    init,
                    crate::gauge::GaugeFieldInit::FromField("other_field".into())
                );
                assert_eq!(seed, None);
                assert!(!persist);
            }
            other => panic!("Expected Statement::GaugeField, got {other:?}"),
        }
    }

    /// TDD-HAL-II.5: declare a lattice, then a gauge field on top of
    /// it; the executor materializes the SU2GaugeField and registers
    /// it in `gauge::registry`, where `get(name)` returns Some.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_executor_registers() {
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl = "LATTICE tdd_hal_ii_5_a FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        let stmt = parse(lat_decl).expect("parse LATTICE");
        execute(&mut engine, &stmt).expect("exec LATTICE");

        let g_decl = "GAUGE_FIELD U_tdd_ii_5_a ON LATTICE tdd_hal_ii_5_a \
                      GROUP SU(2) INIT HAAR_RANDOM SEED 20260616;";
        let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
        match execute(&mut engine, &stmt).expect("exec GAUGE_FIELD") {
            ExecResult::Ok => {}
            other => panic!("expected Ok, got {other:?}"),
        }

        let got = crate::gauge::registry::get("U_tdd_ii_5_a").expect("registered");
        assert_eq!(got.name(), "U_tdd_ii_5_a");
        assert_eq!(got.lattice_name(), "tdd_hal_ii_5_a");
        assert_eq!(got.group(), crate::gauge::Group::SU2);
        let (kind, seed) = got.init_metadata();
        assert_eq!(kind, crate::gauge::GaugeFieldInit::HaarRandom);
        assert_eq!(seed, Some(20260616));
    }

    /// TDD-HAL-II.5: declaring a gauge field on a lattice that was
    /// never declared surfaces a typed error whose Display matches
    /// Halcyon's G2.B regex anchor `not declared`.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_executor_lattice_not_declared() {
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let g_decl = "GAUGE_FIELD U_tdd_ii_5_b ON LATTICE never_declared \
                      GROUP SU(2) INIT IDENTITY;";
        let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
        let err = execute(&mut engine, &stmt).expect_err("expected error");
        assert!(
            err.contains("not declared"),
            "Display must contain 'not declared', got: {err}"
        );
    }

    /// TDD-HAL-II.5: `INIT HAAR_RANDOM` without a SEED clause parses
    /// (the seed is a parser-level Option) but the executor fails
    /// with a typed `SeedRequired` error whose Display contains the
    /// literal "SEED" (Halcyon's `match="SEED"` anchor).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_executor_seed_required() {
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl = "LATTICE tdd_hal_ii_5_c FROM TRUNCATED_ICOSAHEDRON;";
        let stmt = parse(lat_decl).expect("parse LATTICE");
        execute(&mut engine, &stmt).expect("exec LATTICE");

        let g_decl = "GAUGE_FIELD U_tdd_ii_5_c ON LATTICE tdd_hal_ii_5_c \
                      GROUP SU(2) INIT HAAR_RANDOM;";
        let stmt = parse(g_decl).expect("parse GAUGE_FIELD haar no-seed");
        let err = execute(&mut engine, &stmt).expect_err("expected seed-required error");
        assert!(
            err.to_uppercase().contains("SEED"),
            "Display must contain 'SEED', got: {err}"
        );
    }

    /// TDD-HAL-II.5: `GROUP U(1)` parses syntactically but the
    /// executor fails with `UnsupportedGroup` whose Display contains
    /// the literal `SU(2)` — Halcyon's G2.D regex anchor `SU\(2\)`.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_gauge_field_executor_unsupported_group() {
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl = "LATTICE tdd_hal_ii_5_d FROM TRUNCATED_ICOSAHEDRON;";
        let stmt = parse(lat_decl).expect("parse LATTICE");
        execute(&mut engine, &stmt).expect("exec LATTICE");

        let g_decl = "GAUGE_FIELD U_tdd_ii_5_d ON LATTICE tdd_hal_ii_5_d \
                      GROUP U(1) INIT IDENTITY;";
        let stmt = parse(g_decl).expect("parse U(1)");
        let err = execute(&mut engine, &stmt).expect_err("expected unsupported-group error");
        assert!(
            err.contains("SU(2)"),
            "Display must contain 'SU(2)', got: {err}"
        );
    }

    /// TDD-HAL-II.5: `SHOW GAUGE_FIELD U;` (no BUFFER) returns a
    /// single-row result whose metadata columns round-trip the
    /// original GAUGE_FIELD declaration. `SHOW GAUGE_FIELD U BUFFER;`
    /// additionally carries the full (n_edges × repr_dim) buffer in
    /// the `data` column.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_ii_5_show_gauge_field() {
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl = "LATTICE tdd_hal_ii_5_e FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        let stmt = parse(lat_decl).expect("parse LATTICE");
        execute(&mut engine, &stmt).expect("exec LATTICE");

        let g_decl = "GAUGE_FIELD U_tdd_ii_5_e ON LATTICE tdd_hal_ii_5_e \
                      GROUP SU(2) INIT HAAR_RANDOM SEED 20260616;";
        let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
        execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

        // SHOW (no BUFFER) — metadata only.
        let show = "SHOW GAUGE_FIELD U_tdd_ii_5_e;";
        let stmt = parse(show).expect("parse SHOW GAUGE_FIELD");
        let rows = match execute(&mut engine, &stmt).expect("exec SHOW") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        match r.get("name") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "U_tdd_ii_5_e"),
            other => panic!("missing/wrong name column: {other:?}"),
        }
        match r.get("lattice") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "tdd_hal_ii_5_e"),
            other => panic!("missing/wrong lattice column: {other:?}"),
        }
        match r.get("group") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "SU(2)"),
            other => panic!("missing/wrong group column: {other:?}"),
        }
        match r.get("repr_dim") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 4),
            other => panic!("missing/wrong repr_dim column: {other:?}"),
        }
        match r.get("n_edges") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 90),
            other => panic!("missing/wrong n_edges column: {other:?}"),
        }
        match r.get("init_kind") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "HAAR_RANDOM"),
            other => panic!("missing/wrong init_kind column: {other:?}"),
        }
        match r.get("init_seed") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 20260616),
            other => panic!("missing/wrong init_seed column: {other:?}"),
        }
        // No BUFFER form does not carry the data column.
        assert!(r.get("data").is_none(), "metadata-only SHOW must omit data");

        // SHOW … BUFFER — same metadata + the full row-major
        // (n_edges × repr_dim) buffer.
        let show_buf = "SHOW GAUGE_FIELD U_tdd_ii_5_e BUFFER;";
        let stmt = parse(show_buf).expect("parse SHOW GAUGE_FIELD BUFFER");
        let rows = match execute(&mut engine, &stmt).expect("exec SHOW BUFFER") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        match r.get("data") {
            Some(crate::types::Value::Vector(v)) => assert_eq!(v.len(), 90 * 4),
            other => panic!("missing/wrong data column: {other:?}"),
        }
        match r.get("data_flat_len") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 360),
            other => panic!("missing/wrong data_flat_len column: {other:?}"),
        }
    }

    // ─── TDD-HAL-III.6 — GIBBS_SAMPLE + PLAQUETTE + Q_SURROGATE GQL ──
    //
    // Closes the parser + embedded-executor half of Part III's three
    // user-facing verbs. Bee's locked decisions wired through:
    //   D3 (sequential edge order) — III.5 owns; we only exercise the
    //       handle round-trip here.
    //   D4 (registry mutability)   — `gibbs_sample` reaches into the
    //       SU(2)-mut registry through `get_su2_mut(name)`; the parser
    //       arm stays group-agnostic.
    //   D5 (/v1/gql soft-edge)     — the parser arm is the documented
    //       reach-path; HTTP route-table enforcement is the
    //       complementary half (Part IV+).
    //   D6 (Q_SURROGATE shape)     — scalar f64 in the Rows envelope.
    //   D7 (PLAQUETTE shape)       — per_face = Vec<f64> of length F;
    //       mean / sum reductions return scalar f64.

    /// TDD-HAL-III.6: parser accepts the full GIBBS_SAMPLE form
    /// (`BETA β N_SWEEPS n MEASURE_EVERY m MEASURE (…) SEED s`) and
    /// emits a Statement::GibbsSample with every field populated.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iii_6_parse_gibbs_sample_full_form() {
        let stmt = parse(
            "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1 \
             MEASURE (MEAN(PLAQUETTE), Q_SURROGATE) SEED 20260616;",
        )
        .expect("parse GIBBS_SAMPLE full form");
        match stmt {
            Statement::GibbsSample {
                field,
                beta,
                n_sweeps,
                measure_every,
                measure,
                seed,
            } => {
                assert_eq!(field, "U");
                assert_eq!(beta, 2.5);
                assert_eq!(n_sweeps, 200);
                assert_eq!(measure_every, 1);
                assert_eq!(
                    measure,
                    vec![
                        crate::gauge::ObservableId::MeanPlaquette,
                        crate::gauge::ObservableId::QSurrogate,
                    ]
                );
                assert_eq!(seed, Some(20260616));
            }
            other => panic!("Expected Statement::GibbsSample, got {other:?}"),
        }
    }

    /// TDD-HAL-III.6: parser accepts the minimal GIBBS_SAMPLE form
    /// (`BETA β SEED s`) and defaults `n_sweeps = 1`,
    /// `measure_every = 1`, `measure = []`.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iii_6_parse_gibbs_sample_minimal() {
        let stmt = parse("GIBBS_SAMPLE U BETA 2.5 SEED 1;")
            .expect("parse GIBBS_SAMPLE minimal");
        match stmt {
            Statement::GibbsSample {
                field,
                beta,
                n_sweeps,
                measure_every,
                measure,
                seed,
            } => {
                assert_eq!(field, "U");
                assert_eq!(beta, 2.5);
                assert_eq!(n_sweeps, 1);
                assert_eq!(measure_every, 1);
                assert!(measure.is_empty());
                assert_eq!(seed, Some(1));
            }
            other => panic!("Expected Statement::GibbsSample, got {other:?}"),
        }
    }

    /// TDD-HAL-III.6: parser desugars the three SELECT PLAQUETTE
    /// projection shapes into distinct PlaquetteReduction variants.
    /// `SELECT PLAQUETTE OF U;` → PerFace.
    /// `SELECT MEAN(PLAQUETTE OF U);` → Mean.
    /// `SELECT SUM(PLAQUETTE OF U);` → Sum.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iii_6_parse_plaquette_reductions() {
        let per = parse("SELECT PLAQUETTE OF U;").expect("parse PLAQUETTE per-face");
        match per {
            Statement::SelectPlaquette { field, reduction } => {
                assert_eq!(field, "U");
                assert_eq!(reduction, crate::gauge::PlaquetteReduction::PerFace);
            }
            other => panic!("Expected SelectPlaquette PerFace, got {other:?}"),
        }
        let mean = parse("SELECT MEAN(PLAQUETTE OF U);").expect("parse MEAN(PLAQUETTE)");
        match mean {
            Statement::SelectPlaquette { field, reduction } => {
                assert_eq!(field, "U");
                assert_eq!(reduction, crate::gauge::PlaquetteReduction::Mean);
            }
            other => panic!("Expected SelectPlaquette Mean, got {other:?}"),
        }
        let sum = parse("SELECT SUM(PLAQUETTE OF U);").expect("parse SUM(PLAQUETTE)");
        match sum {
            Statement::SelectPlaquette { field, reduction } => {
                assert_eq!(field, "U");
                assert_eq!(reduction, crate::gauge::PlaquetteReduction::Sum);
            }
            other => panic!("Expected SelectPlaquette Sum, got {other:?}"),
        }
    }

    /// TDD-HAL-III.6: end-to-end smoke — declare a buckyball lattice,
    /// register a GAUGE_FIELD U INIT IDENTITY on top, then execute
    /// GIBBS_SAMPLE for 5 sweeps measuring MEAN(PLAQUETTE) every
    /// sweep. The response row carries a 5-entry measurement chain
    /// under `MeanPlaquette` plus the seed echo in diagnostics.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iii_6_execute_gibbs_sample_smoke() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl =
            "LATTICE tdd_hal_iii_6_smoke FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        let stmt = parse(lat_decl).expect("parse LATTICE");
        execute(&mut engine, &stmt).expect("exec LATTICE");

        let g_decl = "GAUGE_FIELD U_iii_6_smoke ON LATTICE tdd_hal_iii_6_smoke \
                      GROUP SU(2) INIT IDENTITY;";
        let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
        execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

        // GIBBS_SAMPLE through the parser → registry must be SU(2)-mut
        // so the III.5 sweep can lock the field. The GAUGE_FIELD arm
        // registers through `register` (Arc<dyn>); we need
        // `register_su2` for the mutable handle. Re-publish into the
        // SU(2)-mut registry explicitly, naming the lattice to match
        // the LATTICE declaration above so `gibbs_sample`'s lattice
        // lookup hits the right entry.
        let handle = crate::gauge::registry::get("U_iii_6_smoke").expect("dyn handle");
        assert_eq!(handle.lattice_name(), "tdd_hal_iii_6_smoke");
        let lat = crate::lattice::registry::get("tdd_hal_iii_6_smoke")
            .expect("declared lattice");
        let su2 = crate::gauge::SU2GaugeField::new(
            "U_iii_6_smoke".into(),
            &lat,
            crate::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        crate::gauge::registry::register_su2(su2);

        let sample = "GIBBS_SAMPLE U_iii_6_smoke BETA 2.5 N_SWEEPS 5 MEASURE_EVERY 1 \
                      MEASURE (MEAN(PLAQUETTE)) SEED 20260616;";
        let stmt = parse(sample).expect("parse GIBBS_SAMPLE");
        let rows = match execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        match r.get("field") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "U_iii_6_smoke"),
            other => panic!("missing/wrong field column: {other:?}"),
        }
        match r.get("seed") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 20260616),
            other => panic!("missing/wrong seed column: {other:?}"),
        }
        match r.get("n_sweeps_completed") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 5),
            other => panic!("missing/wrong n_sweeps_completed: {other:?}"),
        }
        let label = crate::gauge::ObservableId::MeanPlaquette.label();
        match r.get(label) {
            Some(crate::types::Value::Vector(v)) => assert_eq!(
                v.len(),
                5,
                "MeanPlaquette chain length must match n_sweeps/measure_every"
            ),
            other => panic!("missing/wrong {label} column: {other:?}"),
        }
    }

    /// TDD-HAL-III.6: parser accepts a SEED-less GIBBS_SAMPLE form,
    /// but the executor surfaces the typed `SeedRequired` error whose
    /// Display contains the literal "SEED" (locked decision 1 — every
    /// GIBBS_SAMPLE call must commit to a seed for intra-binding
    /// bit-identity).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iii_6_execute_gibbs_sample_no_seed_errors() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl =
            "LATTICE tdd_hal_iii_6_noseed FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        let stmt = parse(lat_decl).expect("parse LATTICE");
        execute(&mut engine, &stmt).expect("exec LATTICE");

        let bb = crate::lattice::topology::truncated_icosahedron::buckyball();
        let su2 = crate::gauge::SU2GaugeField::new(
            "U_iii_6_noseed".into(),
            &bb,
            crate::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        crate::gauge::registry::register_su2(su2);

        // SEED omitted; parser accepts, executor must fail.
        let sample = "GIBBS_SAMPLE U_iii_6_noseed BETA 2.5 N_SWEEPS 5 \
                      MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE));";
        let stmt = parse(sample).expect("parse GIBBS_SAMPLE no-seed");
        let err =
            execute(&mut engine, &stmt).expect_err("expected seed-required error");
        assert!(
            err.to_uppercase().contains("SEED"),
            "Display must contain 'SEED', got: {err}"
        );
    }

    // ─── TDD-HAL-IV.7 — E_FIELD / SYMPLECTIC_FLOW / SHOW E_FIELD /
    //     SELECT H_TOTAL / SELECT GAUSS_RESIDUAL_MAX GQL surface ────
    //
    // Closes the parser + embedded-executor half of the Part IV
    // user-facing verbs. Bee's locked decisions wired through:
    //   IV-A — PROJECT_GAUSS struct-literal sugar with named-field
    //          overrides against the Halcyon production defaults
    //          (tikhonov=1e-14, cg_tol=1e-10, cg_max_iter=200).
    //   IV-B — Sibling SU2EField; parser stays group-agnostic, the
    //          executor matches on `handle.group()` and dispatches
    //          into the SU(2) E-field sibling kernels.
    //   IV-D — PROJECT_GAUSS TRUE → per-step projection cadence
    //          (default config); PROJECT_GAUSS FALSE → None (skip).
    //   IV-I — E_FIELD is embedded-only at this gate; no HTTP route.
    //   IV-J — SELECT H_TOTAL OF (U, E) is the positive-case anchor
    //          (reverses Part III's PartIvObservableNotReady stub at
    //          the SELECT layer). The gibbs_sample stub stays in
    //          place because GIBBS_SAMPLE has no E.

    /// TDD-HAL-IV.7: parser accepts `E_FIELD E ON GAUGE_FIELD U INIT
    /// ZERO;` and emits a `Statement::EField` with init=Zero, seed=None.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_parse_e_field_zero() {
        let stmt = parse("E_FIELD E ON GAUGE_FIELD U INIT ZERO;")
            .expect("parse E_FIELD ZERO");
        match stmt {
            Statement::EField {
                name,
                source_gauge_field,
                init,
                seed,
            } => {
                assert_eq!(name, "E");
                assert_eq!(source_gauge_field, "U");
                assert_eq!(init, crate::gauge::EFieldInit::Zero);
                assert_eq!(seed, None);
            }
            other => panic!("Expected Statement::EField Zero, got {other:?}"),
        }
    }

    /// TDD-HAL-IV.7: parser accepts `E_FIELD E ON GAUGE_FIELD U INIT
    /// MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;` and emits a
    /// `Statement::EField` carrying the β and seed.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_parse_e_field_mb_seed() {
        let stmt = parse(
            "E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;",
        )
        .expect("parse E_FIELD MAXWELL_BOLTZMANN");
        match stmt {
            Statement::EField {
                name,
                source_gauge_field,
                init,
                seed,
            } => {
                assert_eq!(name, "E");
                assert_eq!(source_gauge_field, "U");
                assert_eq!(
                    init,
                    crate::gauge::EFieldInit::MaxwellBoltzmann { beta: 2.5 }
                );
                assert_eq!(seed, Some(20260617));
            }
            other => panic!("Expected Statement::EField MB, got {other:?}"),
        }
    }

    /// TDD-HAL-IV.7: parser accepts `E_FIELD E2 ON GAUGE_FIELD U INIT
    /// FROM other_e;` and emits a `Statement::EField` with
    /// init=FromField("other_e").
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_parse_e_field_from() {
        let stmt = parse("E_FIELD E2 ON GAUGE_FIELD U INIT FROM other_e;")
            .expect("parse E_FIELD FROM");
        match stmt {
            Statement::EField {
                name,
                source_gauge_field,
                init,
                seed,
            } => {
                assert_eq!(name, "E2");
                assert_eq!(source_gauge_field, "U");
                assert_eq!(
                    init,
                    crate::gauge::EFieldInit::FromField("other_e".into())
                );
                assert_eq!(seed, None);
            }
            other => panic!("Expected Statement::EField FromField, got {other:?}"),
        }
    }

    /// TDD-HAL-IV.7: parser accepts the full SYMPLECTIC_FLOW form with
    /// PROJECT_GAUSS TRUE (default config), measure cadence, observable
    /// list, and seed.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_parse_sympflow_full() {
        let stmt = parse(
            "SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 N_STEPS 1000 \
             PROJECT_GAUSS TRUE MEASURE_EVERY 20 \
             MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, Q_SURROGATE) \
             SEED 20260617;",
        )
        .expect("parse SYMPLECTIC_FLOW full");
        match stmt {
            Statement::SymplecticFlow {
                field,
                e_field,
                beta,
                dt,
                n_steps,
                project_gauss,
                measure_every,
                measure,
                seed,
            } => {
                assert_eq!(field, "U");
                assert_eq!(e_field, "E");
                assert_eq!(beta, 2.5);
                assert_eq!(dt, 0.02);
                assert_eq!(n_steps, 1000);
                assert_eq!(
                    project_gauss,
                    Some(crate::gauge::ProjectGaussConfig::default())
                );
                let default = crate::gauge::ProjectGaussConfig::default();
                let cfg = project_gauss.expect("PROJECT_GAUSS TRUE → Some(default)");
                assert_eq!(cfg.tikhonov, default.tikhonov);
                assert_eq!(cfg.cg_tol, default.cg_tol);
                assert_eq!(cfg.cg_max_iter, default.cg_max_iter);
                assert_eq!(measure_every, 20);
                assert_eq!(
                    measure,
                    vec![
                        crate::gauge::ObservableId::HTotal,
                        crate::gauge::ObservableId::MeanPlaquette,
                        crate::gauge::ObservableId::GaussResidualMax,
                        crate::gauge::ObservableId::QSurrogate,
                    ]
                );
                assert_eq!(seed, Some(20260617));
            }
            other => panic!("Expected Statement::SymplecticFlow, got {other:?}"),
        }
    }

    /// TDD-HAL-IV.7: parser accepts the struct-literal PROJECT_GAUSS
    /// override (`{ tikhonov: 1e-12, cg_max_iter: 500 }`) and merges
    /// the named fields against `ProjectGaussConfig::default()`:
    /// tikhonov=1e-12 (overridden), cg_max_iter=500 (overridden),
    /// cg_tol=1e-10 (default).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_parse_sympflow_struct_override() {
        let stmt = parse(
            "SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 N_STEPS 10 \
             PROJECT_GAUSS { tikhonov: 1e-12, cg_max_iter: 500 } SEED 1;",
        )
        .expect("parse SYMPLECTIC_FLOW struct override");
        match stmt {
            Statement::SymplecticFlow {
                project_gauss,
                ..
            } => {
                let cfg =
                    project_gauss.expect("struct-literal PROJECT_GAUSS → Some(cfg)");
                assert_eq!(cfg.tikhonov, 1e-12, "tikhonov overridden to 1e-12");
                assert_eq!(cfg.cg_max_iter, 500, "cg_max_iter overridden to 500");
                assert_eq!(
                    cfg.cg_tol, 1e-10,
                    "cg_tol kept at Halcyon production default"
                );
            }
            other => panic!("Expected Statement::SymplecticFlow, got {other:?}"),
        }
    }

    /// TDD-HAL-IV.7: parser accepts `PROJECT_GAUSS FALSE` and emits
    /// `project_gauss = None` (skip projection).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_parse_sympflow_project_false() {
        let stmt = parse(
            "SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 N_STEPS 1 \
             PROJECT_GAUSS FALSE SEED 1;",
        )
        .expect("parse SYMPLECTIC_FLOW PROJECT_GAUSS FALSE");
        match stmt {
            Statement::SymplecticFlow { project_gauss, .. } => {
                assert!(
                    project_gauss.is_none(),
                    "PROJECT_GAUSS FALSE → None (skip projection)"
                );
            }
            other => panic!("Expected Statement::SymplecticFlow, got {other:?}"),
        }
    }

    /// TDD-HAL-IV.7: end-to-end smoke — declare a buckyball lattice +
    /// GAUGE_FIELD U INIT IDENTITY + thermalize (one Gibbs sweep at
    /// β=2.5) + declare E_FIELD E + run SYMPLECTIC_FLOW for 5 steps
    /// measuring four observables every step. The response row carries
    /// 5-entry chains for each requested observable.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_executor_e_field_then_sympflow() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        crate::gauge::registry::clear_e_registry();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        // Declare a buckyball lattice and a U gauge field on top.
        let lat_decl =
            "LATTICE tdd_hal_iv_7_smoke FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        execute(&mut engine, &parse(lat_decl).expect("parse LATTICE"))
            .expect("exec LATTICE");
        let g_decl =
            "GAUGE_FIELD U_iv_7_smoke ON LATTICE tdd_hal_iv_7_smoke \
             GROUP SU(2) INIT IDENTITY;";
        execute(&mut engine, &parse(g_decl).expect("parse GAUGE_FIELD"))
            .expect("exec GAUGE_FIELD");
        // The GAUGE_FIELD arm registers through the dyn `register`; the
        // SYMPLECTIC_FLOW path needs the SU(2)-mut handle. Republish.
        let bb = crate::lattice::registry::get("tdd_hal_iv_7_smoke")
            .expect("declared lattice");
        let u_field = crate::gauge::SU2GaugeField::new(
            "U_iv_7_smoke".into(),
            &bb,
            crate::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        crate::gauge::registry::register_su2(u_field);

        // Thermalize one Gibbs sweep so the field has non-identity
        // plaquette holonomies. The SYMPLECTIC_FLOW gold-gate
        // (IV.10) drives this on a longer chain; for the smoke we
        // just need the field to move off the trivial start.
        let gibbs = "GIBBS_SAMPLE U_iv_7_smoke BETA 2.5 N_SWEEPS 1 \
                     MEASURE_EVERY 0 SEED 20260617;";
        execute(&mut engine, &parse(gibbs).expect("parse GIBBS_SAMPLE"))
            .expect("exec GIBBS_SAMPLE");

        // Declare the E field via the parser + executor.
        let e_decl = "E_FIELD E_iv_7_smoke ON GAUGE_FIELD U_iv_7_smoke \
                      INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;";
        execute(&mut engine, &parse(e_decl).expect("parse E_FIELD"))
            .expect("exec E_FIELD");

        // Drive SYMPLECTIC_FLOW for 5 steps with per-step projection.
        let flow = "SYMPLECTIC_FLOW U_iv_7_smoke \
                    FROM (U=U_iv_7_smoke, E=E_iv_7_smoke) \
                    BETA 2.5 DT 0.02 N_STEPS 5 PROJECT_GAUSS TRUE \
                    MEASURE_EVERY 1 \
                    MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, Q_SURROGATE) \
                    SEED 20260617;";
        let stmt = parse(flow).expect("parse SYMPLECTIC_FLOW");
        let rows = match execute(&mut engine, &stmt).expect("exec SYMPLECTIC_FLOW") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        match r.get("field") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "U_iv_7_smoke"),
            other => panic!("missing/wrong field column: {other:?}"),
        }
        match r.get("e_field") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "E_iv_7_smoke"),
            other => panic!("missing/wrong e_field column: {other:?}"),
        }
        match r.get("n_steps_completed") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(*n, 5),
            other => panic!("missing/wrong n_steps_completed: {other:?}"),
        }
        for obs in [
            crate::gauge::ObservableId::HTotal,
            crate::gauge::ObservableId::MeanPlaquette,
            crate::gauge::ObservableId::GaussResidualMax,
            crate::gauge::ObservableId::QSurrogate,
        ] {
            let label = obs.label();
            match r.get(label) {
                Some(crate::types::Value::Vector(v)) => assert_eq!(
                    v.len(),
                    5,
                    "{label} chain length must equal n_steps/measure_every"
                ),
                other => panic!("missing/wrong {label} column: {other:?}"),
            }
        }
    }

    /// TDD-HAL-IV.7: `SELECT H_TOTAL OF (U, E);` now reverses the
    /// Part III stub at the SELECT-projection layer (locked decision
    /// IV-J). The positive case returns a finite f64; no
    /// PartIvObservableNotReady error.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_iv_7_executor_select_h_total_now_works() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();
        crate::gauge::registry::clear_e_registry();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl =
            "LATTICE tdd_hal_iv_7_hsel FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        execute(&mut engine, &parse(lat_decl).expect("parse LATTICE"))
            .expect("exec LATTICE");
        let g_decl =
            "GAUGE_FIELD U_iv_7_hsel ON LATTICE tdd_hal_iv_7_hsel \
             GROUP SU(2) INIT IDENTITY;";
        execute(&mut engine, &parse(g_decl).expect("parse GAUGE_FIELD"))
            .expect("exec GAUGE_FIELD");
        // Republish under the SU(2)-mut registry so the SELECT
        // H_TOTAL executor can lock the U handle.
        let bb = crate::lattice::registry::get("tdd_hal_iv_7_hsel")
            .expect("declared lattice");
        let u_field = crate::gauge::SU2GaugeField::new(
            "U_iv_7_hsel".into(),
            &bb,
            crate::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        crate::gauge::registry::register_su2(u_field);

        let e_decl = "E_FIELD E_iv_7_hsel ON GAUGE_FIELD U_iv_7_hsel \
                      INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;";
        execute(&mut engine, &parse(e_decl).expect("parse E_FIELD"))
            .expect("exec E_FIELD");

        let select = "SELECT H_TOTAL OF (U_iv_7_hsel, E_iv_7_hsel);";
        let stmt = parse(select).expect("parse SELECT H_TOTAL");
        let rows = match execute(&mut engine, &stmt).expect("exec SELECT H_TOTAL") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        match r.get("value") {
            Some(crate::types::Value::Float(h)) => {
                assert!(
                    h.is_finite(),
                    "SELECT H_TOTAL must return a finite f64 (post-IV-J reversal); got {h}"
                );
            }
            other => panic!("missing/wrong value column: {other:?}"),
        }
    }

    // ─── TDD-HAL-V.2 — SNAPSHOT GAUGE_FIELD U PERSIST ───────────────
    //
    // Closes the parser + embedded-executor half of the durable
    // post-thermalization buffer snapshot path (Part V P0). Bee's
    // locked decisions wired through:
    //   D-V-A — WAL op encoding is explicit little-endian (the V.1
    //          payload's `to_le_bytes` already enforces this; the
    //          executor builds the payload via `from_buffer` which
    //          mints the SHA-256 over the same LE bytes).
    //   D-V-B — `/v1/gql` is the only HTTP entry point (the
    //          `try_dispatch_gauge_statement` 14-arm match catches
    //          `Statement::Snapshot` so SNAPSHOT lands here instead
    //          of dropping through the bundle-aware path).
    //   D-V-C — SHA-256 over the LE buffer bytes is the citation
    //          handle; the same hash lands in the WAL entry AND the
    //          Rows envelope returned to the caller.
    //   D-V-D — PERSIST is REQUIRED (not the default). Bare
    //          `SNAPSHOT GAUGE_FIELD U;` parse-errors so the future
    //          TRANSIENT variant ships without flipping any existing
    //          call site.

    /// TDD-HAL-V.2: `SNAPSHOT GAUGE_FIELD U PERSIST;` parses into the
    /// expected Statement::Snapshot shape with `persist = true` (the
    /// red anchor for the gate).
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_2_parse_snapshot_with_persist() {
        let stmt = parse("SNAPSHOT GAUGE_FIELD U PERSIST;")
            .expect("parse SNAPSHOT PERSIST");
        match stmt {
            Statement::Snapshot { name, persist } => {
                assert_eq!(name, "U");
                assert!(persist);
            }
            other => panic!("Expected Statement::Snapshot, got {other:?}"),
        }
    }

    /// TDD-HAL-V.2: bare `SNAPSHOT GAUGE_FIELD U;` (no PERSIST clause)
    /// parse-errors per Bee's locked decision D-V-D. The error must
    /// mention "PERSIST" so the future TRANSIENT variant can ship
    /// without flipping any existing call site.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_2_parse_snapshot_bare_rejected() {
        let err = parse("SNAPSHOT GAUGE_FIELD U;")
            .expect_err("bare SNAPSHOT GAUGE_FIELD must parse-error");
        assert!(
            err.contains("PERSIST"),
            "parse error must name the missing PERSIST keyword, got: {err}"
        );
    }

    /// TDD-HAL-V.2: end-to-end smoke — declare a buckyball lattice,
    /// register a GAUGE_FIELD U INIT IDENTITY on top, run a few
    /// thermalization sweeps, then execute SNAPSHOT GAUGE_FIELD U
    /// PERSIST. The response row carries `field`, `n_edges` (90 for
    /// SU(2) on the buckyball), `repr_dim` (4), the SHA-256 hex, and
    /// the post-write WAL offset.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_2_executor_snapshot_succeeds() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl =
            "LATTICE tdd_hal_v_2_smoke FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        execute(&mut engine, &parse(lat_decl).expect("parse LATTICE"))
            .expect("exec LATTICE");

        let g_decl =
            "GAUGE_FIELD U_v_2_smoke ON LATTICE tdd_hal_v_2_smoke \
             GROUP SU(2) INIT IDENTITY;";
        execute(&mut engine, &parse(g_decl).expect("parse GAUGE_FIELD"))
            .expect("exec GAUGE_FIELD");

        // Thermalization: GIBBS_SAMPLE mutates the buffer so the
        // snapshot captures non-identity state.
        let sample = "GIBBS_SAMPLE U_v_2_smoke BETA 2.5 N_SWEEPS 5 SEED 20260616;";
        execute(&mut engine, &parse(sample).expect("parse GIBBS_SAMPLE"))
            .expect("exec GIBBS_SAMPLE");

        let snap = "SNAPSHOT GAUGE_FIELD U_v_2_smoke PERSIST;";
        let stmt = parse(snap).expect("parse SNAPSHOT");
        let rows = match execute(&mut engine, &stmt).expect("exec SNAPSHOT") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1, "SNAPSHOT must return one row");
        let r = &rows[0];
        match r.get("field") {
            Some(crate::types::Value::Text(s)) => assert_eq!(s, "U_v_2_smoke"),
            other => panic!("missing/wrong field column: {other:?}"),
        }
        match r.get("n_edges") {
            Some(crate::types::Value::Integer(n)) => assert_eq!(
                *n, 90,
                "buckyball has 90 edges"
            ),
            other => panic!("missing/wrong n_edges column: {other:?}"),
        }
        match r.get("repr_dim") {
            Some(crate::types::Value::Integer(d)) => assert_eq!(
                *d, 4,
                "SU(2) repr_dim is 4"
            ),
            other => panic!("missing/wrong repr_dim column: {other:?}"),
        }
        match r.get("sha256") {
            Some(crate::types::Value::Text(hex)) => {
                assert_eq!(hex.len(), 64, "SHA-256 hex is 64 chars");
                assert!(
                    hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "SHA-256 hex must be lowercase, got: {hex}"
                );
            }
            other => panic!("missing/wrong sha256 column: {other:?}"),
        }
        match r.get("wal_offset") {
            Some(crate::types::Value::Integer(o)) => {
                assert!(*o > 0, "wal_offset must be positive after write");
            }
            other => panic!("missing/wrong wal_offset column: {other:?}"),
        }
    }

    /// TDD-HAL-V.2: SNAPSHOT against an undeclared field surfaces the
    /// typed `FieldNotDeclared` Display so the regex anchor
    /// "is not declared" matches.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_2_executor_snapshot_undeclared_field() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let snap = "SNAPSHOT GAUGE_FIELD nonexistent PERSIST;";
        let stmt = parse(snap).expect("parse SNAPSHOT");
        let err = execute(&mut engine, &stmt)
            .expect_err("undeclared field must surface FieldNotDeclared");
        assert!(
            err.contains("'nonexistent'") && err.contains("is not declared"),
            "error must say `field 'nonexistent' is not declared`, got: {err}"
        );
    }

    /// TDD-HAL-V.2: SNAPSHOT twice against the same IDENTITY buffer
    /// returns the same SHA-256 both times. The identity buffer is
    /// deterministic by construction, so the citation handle is
    /// content-addressed: equal buffer bytes → equal SHA-256.
    #[cfg(feature = "gauge")]
    #[test]
    fn tdd_hal_v_2_executor_snapshot_sha256_deterministic() {
        let _g = crate::gauge::registry::test_serial_lock();
        crate::lattice::registry::clear();
        crate::gauge::registry::clear();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = crate::engine::Engine::open(dir.path()).expect("engine open");

        let lat_decl =
            "LATTICE tdd_hal_v_2_det FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        execute(&mut engine, &parse(lat_decl).expect("parse LATTICE"))
            .expect("exec LATTICE");
        let g_decl =
            "GAUGE_FIELD U_v_2_det ON LATTICE tdd_hal_v_2_det \
             GROUP SU(2) INIT IDENTITY;";
        execute(&mut engine, &parse(g_decl).expect("parse GAUGE_FIELD"))
            .expect("exec GAUGE_FIELD");

        let snap = "SNAPSHOT GAUGE_FIELD U_v_2_det PERSIST;";
        let stmt = parse(snap).expect("parse SNAPSHOT");

        let rows_a = match execute(&mut engine, &stmt).expect("exec SNAPSHOT #1") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        let rows_b = match execute(&mut engine, &stmt).expect("exec SNAPSHOT #2") {
            ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        let sha_a = match rows_a[0].get("sha256") {
            Some(crate::types::Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong sha256 in #1: {other:?}"),
        };
        let sha_b = match rows_b[0].get("sha256") {
            Some(crate::types::Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong sha256 in #2: {other:?}"),
        };
        assert_eq!(
            sha_a, sha_b,
            "IDENTITY buffer SHA-256 must be deterministic across SNAPSHOT calls"
        );
        // The two SNAPSHOTs land at consecutive WAL offsets — a
        // necessary (but not sufficient) sign that both calls wrote.
        let off_a = match rows_a[0].get("wal_offset") {
            Some(crate::types::Value::Integer(n)) => *n,
            other => panic!("missing wal_offset in #1: {other:?}"),
        };
        let off_b = match rows_b[0].get("wal_offset") {
            Some(crate::types::Value::Integer(n)) => *n,
            other => panic!("missing wal_offset in #2: {other:?}"),
        };
        assert!(
            off_b > off_a,
            "second SNAPSHOT must land at a higher WAL offset; got {off_a} → {off_b}"
        );
    }
}
